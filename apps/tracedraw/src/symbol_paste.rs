//! Symbol instances pasted into a drawing: each instance names its
//! symbol by its place in the drawing's symbol list, so instances copied
//! in another drawing (or after that list changed) are pointed at the
//! same symbol here, which is added when the drawing lacks it (#678).

use tracedraw_core::document::{Shape, ShapeKind, Symbol};

/// Symbols nested deeper than this are left as they are.
const MAX_DEPTH: usize = 16;

/// Point the instances in `shape` (inside groups, clip frames and the
/// symbols they show) at the symbols of `here`, the drawing's symbol list
/// as it will be: a symbol of `from` (the list of the drawing copied
/// from) equal to one of `here` uses it, else it is appended to `here`.
/// An instance of a symbol `from` does not hold, or of a symbol that
/// shows itself, is left as it was.
pub fn bring(shape: &mut Shape, from: &[Symbol], here: &mut Vec<Symbol>) {
    walk(shape, from, here, &mut Vec::new());
}

fn walk(shape: &mut Shape, from: &[Symbol], here: &mut Vec<Symbol>, open: &mut Vec<usize>) {
    if open.len() >= MAX_DEPTH {
        return;
    }
    match &mut shape.kind {
        ShapeKind::SymbolInstance { index } => {
            if let Some(i) = symbol_here(*index, from, here, open) {
                *index = i;
            }
        }
        ShapeKind::Group { children } => {
            for c in children {
                walk(c, from, here, open);
            }
        }
        ShapeKind::ClipFrame { frame, contents } => {
            walk(frame, from, here, open);
            for c in contents {
                walk(c, from, here, open);
            }
        }
        _ => {}
    }
}

/// The index in `here` of symbol `index` of `from`, added when missing.
/// `open` holds the symbols being brought, to stop at one that shows
/// itself.
fn symbol_here(
    index: usize,
    from: &[Symbol],
    here: &mut Vec<Symbol>,
    open: &mut Vec<usize>,
) -> Option<usize> {
    let sym = from.get(index)?;
    // The same drawing (or one with the same list so far): the same place.
    if here.get(index) == Some(sym) {
        return Some(index);
    }
    if open.contains(&index) {
        return None;
    }
    // The symbol as this drawing would hold it, its own instances pointed
    // at this drawing's symbols too.
    let mut def = sym.clone();
    open.push(index);
    walk(&mut def.shape, from, here, open);
    open.pop();
    if let Some(i) = here.iter().position(|s| *s == def) {
        return Some(i);
    }
    here.push(def);
    Some(here.len() - 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::ShapeId;

    fn rect(w: f64) -> Shape {
        Shape::new(
            ShapeId(0),
            ShapeKind::Rect {
                rect: kurbo::Rect::new(0.0, 0.0, w, 1.0),
                radius: 0.0,
                corners: None,
            },
        )
    }

    fn instance(index: usize) -> Shape {
        Shape::new(ShapeId(1), ShapeKind::SymbolInstance { index })
    }

    fn sym(name: &str, shape: Shape) -> Symbol {
        Symbol {
            name: name.into(),
            shape,
        }
    }

    fn index_of(s: &Shape) -> usize {
        match s.kind {
            ShapeKind::SymbolInstance { index } => index,
            _ => panic!("not an instance"),
        }
    }

    /// A symbol that shows another one comes with it, its instance
    /// pointed at the other's place here.
    #[test]
    fn nested_symbols_come_along() {
        let from = vec![sym("Inner", rect(2.0)), sym("Outer", instance(0))];
        let mut here = vec![sym("Own", rect(9.0))];
        let mut s = instance(1);
        bring(&mut s, &from, &mut here);
        assert_eq!(here.len(), 3);
        assert_eq!(here[1].name, "Inner");
        assert_eq!(here[2].name, "Outer");
        assert_eq!(index_of(&here[2].shape), 1);
        assert_eq!(index_of(&s), 2);
        // Brought again: nothing new.
        let mut again = instance(1);
        bring(&mut again, &from, &mut here);
        assert_eq!(here.len(), 3);
        assert_eq!(index_of(&again), 2);
    }

    /// An instance of a symbol the list does not hold stays as it was; a
    /// symbol that shows itself comes once, without looping.
    #[test]
    fn missing_and_self_nested_symbols_stay() {
        let from = vec![sym("Self", instance(0))];
        let mut here = vec![sym("Own", rect(9.0))];
        let mut s = instance(5);
        bring(&mut s, &from, &mut here);
        assert_eq!(index_of(&s), 5);
        assert_eq!(here.len(), 1);
        let mut s = instance(0);
        bring(&mut s, &from, &mut here);
        assert_eq!(here.len(), 2);
        assert_eq!(index_of(&s), 1);
    }

    /// Inside groups and clip frames too.
    #[test]
    fn instances_inside_groups_are_pointed_too() {
        let from = vec![sym("A", rect(1.0))];
        let mut here = vec![sym("B", rect(3.0))];
        let mut g = Shape::new(
            ShapeId(2),
            ShapeKind::Group {
                children: vec![instance(0), rect(4.0)],
            },
        );
        bring(&mut g, &from, &mut here);
        let ShapeKind::Group { children } = &g.kind else {
            panic!("group")
        };
        assert_eq!(index_of(&children[0]), 1);
        assert_eq!(here.len(), 2);
    }
}
