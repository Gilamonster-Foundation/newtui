Output demos/linked_panes.gif

Set Shell "bash"
Set Width 1280
Set Height 760
Set FontSize 16
Set FontFamily "Menlo"
Set Theme "Catppuccin Mocha"
Set CursorBlink false
Set Padding 16

Hide
Type `"${NEWTUI_DEMO_BIN:-./target/debug/examples/demo}" linked_panes`
Enter
Wait+Screen /Locked \/ normal/
Sleep 500ms
Show
Sleep 1s
Down 2
Sleep 1s
Tab
Sleep 800ms
Tab
Down 4
Sleep 1s
Down 2
Sleep 1s
Type "l"
Sleep 1s
Type "l"
Up 2
Sleep 1s
Type "l"
Sleep 1s
Left 3
Sleep 1s
Right 3
Type "f"
Sleep 1s
Type "f"
Sleep 1s
Type "f"
Sleep 1s
Type "f"
PageDown 2
Sleep 1s
Type "x"
Sleep 500ms
Escape
Sleep 1s
Type "r"
Sleep 500ms
Type "q"
Sleep 200ms
