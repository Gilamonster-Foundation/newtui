# Capture reproduction inputs

Generated from the real terminal host using the checked-in tapes below.

Source revision: 0765f116378dc0faee16a5d02edd09803faf121f
Recorder: vhs version 0.12.1
Working tree: includes the reviewed changes accompanying these captures.

## demos/modal.tape

```text
Output demos/modal.gif

Set Shell "bash"
Set Width 1280
Set Height 760
Set FontSize 16
Set FontFamily "Menlo"
Set Theme "Catppuccin Mocha"
Set CursorBlink false
Set Padding 16

Hide
Type `"${NEWTUI_DEMO_BIN:-./target/debug/examples/demo}" modal`
Enter
Wait+Screen /modal size \/ sized \/ normal/
Sleep 500ms
Show
Sleep 2s
Type "+"
Sleep 700ms
Type "+"
Sleep 1s
Type@700ms "----"
Sleep 1s
Type@700ms "---"
Sleep 1s
Type "z"
Sleep 2s
Type "z"
Sleep 1.5s
Type@700ms "+++"
Sleep 1s
Up
Sleep 1s
Left
Sleep 800ms
Left
Sleep 800ms
Right 2
Sleep 800ms
Type "f"
Sleep 800ms
Type "f"
Sleep 1.5s
Type "f"
Sleep 800ms
Type "f"
Sleep 2s
Type "-"
Sleep 1.5s
Type "z"
Sleep 1.5s
Type "z"
Sleep 1.5s
Type "q"
Sleep 200ms
```
