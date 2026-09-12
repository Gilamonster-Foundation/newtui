Output demos/catalog/bsp.gif

Set Shell "bash"
Set Width 1500
Set Height 940
Set FontSize 18
Set FontFamily "Menlo"
Set Theme "Catppuccin Mocha"
Set CursorBlink false
Set Padding 16

Hide
Type `"$NEWTUI_CATALOG_BIN" --item bsp --width 88 --theme dark`
Enter
Wait+Screen /LIVE CATALOG/
Sleep 1s
Show
Enter
Sleep 1s
Screenshot docs/widgets/generated/bsp.png
Sleep 1s
Type "s"
Sleep 1s
Type "s"
Sleep 1s
Tab
Up
Sleep 1s
Down
Sleep 1s
Escape
Type "rf"
Enter
Sleep 1s
Screenshot docs/widgets/generated/bsp-narrow.png
Sleep 1s
Escape
Type "f"
Enter
Sleep 1s
Escape
Type "f"
Enter
Type "x"
Sleep 1s
Screenshot docs/widgets/generated/bsp-error.png
Sleep 1s
Escape
Type "q"
Sleep 200ms
