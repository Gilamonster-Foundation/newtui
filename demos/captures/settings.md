# Capture reproduction inputs

Generated from the real terminal host using the checked-in tapes below.

Source revision: ce793a7239d83417adcd5c2e4bcdf8286aa3e66c
Recorder: vhs version v0.11.0 (c6af91a)
Working tree: includes the reviewed changes accompanying these captures.

## demos/settings.tape

```text
Output demos/settings.gif

Set Shell "bash"
Set Width 780
Set Height 420
Set FontSize 18
Set FontFamily "Menlo"
Set Theme "Catppuccin Mocha"
Set CursorBlink false
Set TypingSpeed 20ms

Hide
Type `"${NEWTUI_DEMO_BIN:-./target/debug/examples/demo}" settings`
Enter
Wait+Screen /tenacity/
Sleep 1s
Show

Sleep 800ms
Right
Sleep 800ms
Down
Sleep 1s
Down
Sleep 800ms
Right
Sleep 800ms
Up
Sleep 500ms
Down
Sleep 500ms
Escape
Sleep 2s
Type "q"
Sleep 300ms
```
