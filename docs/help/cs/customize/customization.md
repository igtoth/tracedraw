# Možnosti přizpůsobení

> *Strojový překlad: tato stránka může zaostávat za anglickou verzí.*

[[menu.tools > menu.tools.options > menu.tools.options_customization]]
nastavuje vzhled rozhraní, vaše vlastní klávesové zkratky, zobrazené lišty a
chování barevné palety.

## Vzhled {#appearance}

![Stránka Vzhled v možnostech přizpůsobení](../../img/customize-customization-appearance.png)

- [[options.center_dialogs]]: dialogová okna se otevírají uprostřed okna; po
  vypnutí se otevírají tam, kde jste je naposledy nechali.
- [[options.ui_shadows]] (méně stínů pomůže pomalejší grafice).
- [[options.colored_icons]]: každá skupina nástrojů dostane vlastní barvu.
- [[options.ui_size]]: jak velké jsou nabídky, lišty, ikony a panely vzhledem
  ke škálování, které systém monitoru dává (výchozí 100 %).
  [[options.reset_to_default]] se vrátí na 100 %. Velikost rozhraní nikdy
  nemění velikost vaší kresby. [[options.follow_text_size]] ji zvětšuje
  spolu s velikostí textu plochy.
- [[options.color_section]]: [[options.theme]] (Světlý, Středně světlý,
  Středně tmavý, Tmavý nebo Vysoký kontrast), [[options.follow_high_contrast]]
  a [[options.desktop_color]] kolem stránky. Tlačítko na pravém konci
  stavového řádku přepíná mezi světlým a tmavým.

## Příkazy {#commands}

![Stránka Příkazy v možnostech přizpůsobení](../../img/customize-customization-commands.png)

Stránka uvádí všechny příkazy nabídek s jejich zkratkami a klávesy nástrojů.
Psaním do pole [[options.find_command]] seznam filtrujete.

Jak dát příkazu vlastní zkratku:

1. Klikněte na tlačítko zkratky příkazu.
2. Stiskněte klávesy, například <kbd>Ctrl+Alt+K</kbd>. <kbd>Esc</kbd> čekání
   ukončí.

Vaše zkratky mají přednost před vestavěnými: když si vezmete klávesu, kterou
používá jiný příkaz, stránka řekne, který příkaz ji ztratil. Nabídky ukazují
vaši zkratku. **x** vedle zkratky ji odebere a obnovení výchozích hodnot
odebere všechny. Klávesy nástrojů jsou písmeno nebo <kbd>Shift</kbd> a
písmeno. Viz [Klávesové zkratky](../reference/keyboard-shortcuts.md).

Příkaz může mít více zkratek: nová klávesa se přidá k těm, které už má
(vestavěná klávesa funguje dál vedle ní), a každá klávesa má vlastní tlačítko
pro odstranění. Zkratky přijímají i tlačítka panelů, která spouštějí příkaz
nabídky (Nový, Otevřít, Uložit, Zpět, Přiblížit a další), se stejnými
klávesami jako příkaz nabídky, a také řádky, které nabídky ukazují jen někdy.
Seznam **Tlačítka panelů nástrojů** obsahuje všechna tlačítka panelů Standardní
a Lupa: tlačítko odebrané z panelu odtud přetáhněte zpět.

## Příkazové lišty {#command-bars}

![Stránka Příkazové lišty v možnostech přizpůsobení](../../img/customize-customization-command-bars.png)

[[options.command_bars_hint]] [[menu.window.toolbar_menu_bar]],
[[menu.window.toolbar_standard]], [[menu.window.toolbar_property_bar]],
[[menu.window.toolbar_toolbox]], [[menu.window.toolbar_status_bar]],
[[bars.palette]] a panely nástrojů [[menu.window.toolbar_text]],
[[menu.window.toolbar_zoom]] a [[menu.window.toolbar_transform]].
[[options.reset]] vrátí lišty na místo. V části [[options.other]]:
[[bars.lock]] a [[options.show_outline_flyout]].

[[options.toolbar_drag_hint]]

## Barevná paleta {#color-palette}

![Stránka Barevná paleta v možnostech přizpůsobení](../../img/customize-customization-color-palette.png)

- [[options.palette_auto_update]] a [[options.palette_show_document]] (obojí
  zapnuto).
- [[options.palette_swatches]]: [[options.palette_no_color]] (zapnuto) a co
  dělá [[options.palette_right_button]]: [[options.palette_set_outline]]
  (výchozí) nebo otevře [[options.palette_context_menu]].

Viz [Barevné palety](../color/palettes.md).
