# Capture reproduction inputs

Generated from the real terminal host using the checked-in tapes below.

Source revision: ad61c319cf64cee2ad8bf00033b0621e52df9fe2
Recorder: vhs version v0.11.0 (c6af91a)
Working tree: includes the reviewed changes accompanying these captures.

## demos/butterfly.tape

```text
Output demos/butterfly.gif

Set Shell "bash"
Set Width 1320
Set Height 1000
Set FontSize 16
Set FontFamily "Menlo"
Set Theme "Catppuccin Mocha"
Set CursorBlink false
Set Padding 16

Hide
Type `"${NEWTUI_DEMO_BIN:-./target/debug/examples/demo}" butterfly`
Enter
Wait+Screen /SYNTHETIC LIVE/
Sleep 500ms
Show
Sleep 12s
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
```
