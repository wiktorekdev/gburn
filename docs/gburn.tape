# Real terminal capture of gburn demo TUI
Output docs/screenshot.gif
Output docs/screenshot.png

Set Shell "pwsh"
Set FontSize 15
Set FontFamily "Cascadia Mono"
Set Width 1280
Set Height 720
Set Padding 12
Set Theme "Catppuccin Mocha"
Set WindowBar Colorful
Set WindowBarSize 36
Set BorderRadius 8
Set TypingSpeed 1ms
Set Framerate 24

Hide
Type "cd 'C:\Users\sigma\Desktop\Dev\ideas\grok tokens'"
Enter
Type "$env:GBURN_DEMO='1'; $env:FORCE_COLOR='1'"
Enter
Show

Type "bun run src/index.ts"
Enter

Sleep 3s
Down 2
Sleep 500ms
Up 2
Sleep 1s
