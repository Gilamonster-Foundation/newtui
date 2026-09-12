Output demos/catalog/narrow.gif

Set Shell "bash"
Set Width 760
Set Height 720
Set FontSize 18
Set FontFamily "Menlo"
Set Theme "Catppuccin Mocha"
Set CursorBlink false
Set Padding 16

Hide
Type `"$NEWTUI_CATALOG_BIN" --item butterfly --scenario narrow --theme dark`
Enter
Wait+Screen /LIVE CATALOG/
Sleep 1s
Show
Sleep 1s
Screenshot docs/widgets/generated/catalog-narrow.png
Sleep 1s
Type "q"
Sleep 200ms
