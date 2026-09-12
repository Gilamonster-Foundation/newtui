# Capture reproduction inputs

Generated from the real terminal host using the checked-in tapes below.

Source revision: 518184f0e9698a2990b67abc07bfd59cb9e7c7f1
Recorder: vhs version v0.11.0 (c6af91a)
Working tree: includes the reviewed changes accompanying these captures.

## demos/diff.tape

```text
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
```
