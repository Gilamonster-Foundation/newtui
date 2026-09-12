# Capture reproduction inputs

Generated from the real terminal host using the checked-in tapes below.

Source revision: 518184f0e9698a2990b67abc07bfd59cb9e7c7f1
Recorder: vhs version v0.11.0 (c6af91a)
Working tree: includes the reviewed changes accompanying these captures.

## demos/gauge.tape

```text
Output demos/gauge.gif

Set Shell "bash"
Set Width 620
Set Height 260
Set FontSize 18
Set FontFamily "Menlo"
Set Theme "Catppuccin Mocha"
Set CursorBlink false

Hide
Type `"${NEWTUI_DEMO_BIN:-./target/debug/examples/demo}" gauge`
Enter
Wait+Screen /requested width:/
Sleep 1s
Show

Sleep 800ms
Left
Sleep 900ms
Left
Sleep 1200ms
Right
Sleep 700ms
Type "q"
Sleep 300ms
```
