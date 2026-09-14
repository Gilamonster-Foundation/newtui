Output demos/gauge.gif

Set Shell "bash"
Set Width 820
Set Height 360
Set FontSize 16
Set FontFamily "Menlo"
Set Theme "Catppuccin Mocha"
Set CursorBlink false
Set Padding 16

Hide
Type `"${NEWTUI_DEMO_BIN:-./target/debug/examples/demo}" gauge`
Enter
Wait+Screen /SYNTHETIC LIVE/
Sleep 500ms
Show
Sleep 10s
Left
Sleep 1s
Right
Sleep 1s
Type " "
Sleep 800ms
Type "."
Sleep 800ms
Type "ff"
Sleep 800ms
Type "f"
Sleep 800ms
Type "f"
Sleep 800ms
Type "q"
Sleep 200ms
