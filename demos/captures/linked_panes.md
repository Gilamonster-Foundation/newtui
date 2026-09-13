# Capture reproduction inputs

Generated from the real terminal host using the checked-in tapes below.

Source revision: d119ece7672bb980af93a1d03d92c30ec2a4f61c
Recorder: vhs version v0.11.0 (c6af91a)
Working tree: includes the reviewed changes accompanying these captures.

## demos/linked_panes.tape

```text
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
```
