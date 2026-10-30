# Symbols

A **symbol** is an object you define once and place many times. Each copy,
called an **instance**, is a link to the definition: edit the definition and
every instance changes. Symbols keep files small and repeated parts the same.
They are listed in the [[docker.symbols]] panel (<kbd>Ctrl+F3</kbd>).

![The Symbols panel](../../img/objects-symbols.png)

## Making a symbol {#create}

1. Select the objects that make up the symbol.
2. Click [[docker.new_symbol]] in the [[docker.symbols]] panel, or choose
   [[menu.object > menu.object.symbol > menu.object.symbol_new]] or
   [[menu.object > menu.object.create > menu.object.create_symbol]].

The selection becomes the first instance of the new symbol.

## Placing instances {#insert}

The panel lists the symbols of the drawing, each with the number of its
instances. Click [[docker.insert]] beside a symbol to place a new instance, or
choose [[menu.object > menu.object.symbol > menu.object.symbol_insert]].
Instances can be moved, scaled and turned like any object.

An instance copied in one drawing and pasted in another brings its symbol
along (and the symbols it shows), so it shows the same objects there.
Pasting again reuses the symbol already brought.

## Turning instances back into objects {#revert}

Select instances and click [[docker.revert_to_objects]], or choose
[[menu.object > menu.object.symbol > menu.object.symbol_revert]]. They become
ordinary objects that no longer follow the symbol.
