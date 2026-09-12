Output demos/catalog/catalog.gif

Set Shell "bash"
Set Width 1200
Set Height 760
Set FontSize 18
Set FontFamily "Menlo"
Set Theme "Catppuccin Mocha"
Set CursorBlink false
Set Padding 16

Hide
Type `"$NEWTUI_CATALOG_BIN" --item sparkline --scenario long --theme dark`
Enter
Wait+Screen /LIVE CATALOG/
Sleep 1s
Show
Sleep 1s
Screenshot docs/widgets/generated/catalog.png
Sleep 1s
Type "f"
Down
Sleep 800ms
Screenshot docs/widgets/generated/butterfly.png
Sleep 1s
Down
Sleep 800ms
Screenshot docs/widgets/generated/heat-meter.png
Sleep 1s
Down
Sleep 800ms
Screenshot docs/widgets/generated/gauge.png
Sleep 1s
Type "q"
Sleep 200ms
