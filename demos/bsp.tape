Output demos/bsp.gif

Set Shell "bash"
Set Width 1280
Set Height 760
Set FontSize 16
Set FontFamily "Menlo"
Set Theme "Catppuccin Mocha"
Set CursorBlink false
Set Padding 16

Hide
Type `"${NEWTUI_DEMO_BIN:-./target/debug/examples/demo}" bsp`
Enter
Wait+Screen /full width \/ normal/
Sleep 500ms
Show
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
Type "x"
Sleep 1s
Type "r"
Left
Sleep 800ms
Left
Sleep 800ms
Left
Sleep 1s
Right 3
Type "f"
Sleep 800ms
Type "f"
Sleep 800ms
Type "f"
Sleep 1s
Type "q"
Sleep 200ms
