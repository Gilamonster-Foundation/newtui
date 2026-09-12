Output demos/catalog/error.gif

Set Shell "bash"
Set Width 1200
Set Height 760
Set FontSize 18
Set FontFamily "Menlo"
Set Theme "Catppuccin Mocha"
Set CursorBlink false
Set Padding 16

Hide
Type `"$NEWTUI_CATALOG_BIN" --item settings_panel --scenario error --theme dark`
Enter
Wait+Screen /LIVE CATALOG/
Sleep 1s
Show
Sleep 1s
Screenshot docs/widgets/generated/catalog-error.png
Sleep 1s
Type "q"
Sleep 200ms
