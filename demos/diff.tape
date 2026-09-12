Output demos/diff.gif

Set Shell "bash"
Set Width 1280
Set Height 760
Set FontSize 16
Set FontFamily "Menlo"
Set Theme "Catppuccin Mocha"
Set CursorBlink false
Set Padding 16

Hide
Type `"${NEWTUI_DEMO_BIN:-./target/debug/examples/demo}" diff`
Enter
Wait+Screen /unified \/ normal/
Sleep 500ms
Show
Sleep 1s
Type "g"
Sleep 1s
Type "g"
Sleep 1s
Type "g"
Type "e"
Sleep 1s
Down 3
Sleep 800ms
Up 3
Type "e"
Left
Sleep 800ms
Left
Sleep 800ms
Left
Type "n"
Sleep 1s
Right 3
Type "f"
Sleep 800ms
Type "f"
Sleep 800ms
Type "f"
Sleep 800ms
Type "f"
PageDown
Sleep 1s
Type "n"
Sleep 800ms
Type "q"
Sleep 200ms
