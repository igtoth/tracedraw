# Script debugger

Desktop only (the browser build has no worker threads; there the Scripts
panel's Debug button is greyed and its tip says so, #1416).

- The Scripts panel's editor has a margin with the line numbers. A click
  in the margin sets a breakpoint on that line (a red dot) or clears it.
  A wrapped line keeps one number. Breakpoints stay while the app runs
  and can change while a script is paused.
- **Debug** runs the panel's script on a worker thread. Before every
  statement that starts a line the script gets a check, on that same
  line, so line numbers in errors stay the same. Statements that continue
  the line before (a line starting with `(`, `[`, `.`, an operator or a
  template), the statement an `if`, `for`, `while` or `with` head guards
  on the next line, and lines inside object literals, class bodies,
  `switch` cases and template substitutions get no check; a breakpoint
  there stops at the next line that has one. A script that does not
  parse runs without checks (and fails as it would with Run).
- At a breakpoint the panel shows **Paused at line N** and marks the
  line. **Continue** runs to the next breakpoint, **Step** to the next
  statement with a check (into functions too), **Stop** throws an error
  where the script is paused and at every check after it.
- While paused, **Evaluate** (or Enter) runs the expression in the paused
  scope, local variables included, and prints `> expression` and the
  result (or the error) in the output. Printed lines arrive at each
  pause.
- The drawing changes when the script ends, as one undo step, like Run;
  the changes made before an error or a Stop are kept. Run and Debug are
  off while a script is being debugged.

## Checks

- `script_debug::tests::statements_that_start_lines_get_checks`
- `script_debug::tests::objects_classes_switches_and_templates_get_none_inside`
- `script_debug::tests::calls_and_operators_on_the_next_line_are_left_alone`
- `script_debug::tests::instrumented_scripts_run_the_same`
- `script_debug::tests::broken_or_odd_input_is_left_as_it_is`
- `script_debug::tests::breakpoints_pause_and_show_the_local_scope`
- `script_debug::tests::stop_ends_the_script_keeping_what_ran`
- Looked at under Xvfb: a click in the margin sets the dot, Debug pauses
  at it with the line marked, `r.id` evaluates to the new rectangle's id,
  Continue finishes the script.
