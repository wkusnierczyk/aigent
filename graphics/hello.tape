# aigent demo — AI agent skill toolchain
# Run with: cd /tmp && rm -rf aigent hello-world && HOMEBREW_NO_AUTO_UPDATE=1 vhs ~/dev/aigent/graphics/hello.tape

# --- Settings ---
Output aigent/hello.gif
Output aigent/hello.mp4

Set Shell "bash"
Set FontSize 16
Set Width 900
Set Height 500
Set Padding 20
Set Theme "Catppuccin Mocha"
Set TypingSpeed 40ms

# --- Install with cargo ---
Ctrl+L
Set TypingSpeed 0ms
Type "# Install with Cargo from crates.io"
Enter
Set TypingSpeed 40ms
Type "cargo install aigent"
Enter
Sleep 5s

# --- Install with Homebrew ---
Ctrl+L
Set TypingSpeed 0ms
Type "# Install with Homebrew"
Enter
Set TypingSpeed 40ms
Type "brew install wkusnierczyk/aigent/aigent"
Enter
Sleep 5s

# --- Print about ---
Ctrl+L
Set TypingSpeed 0ms
Type "# Print tool stanza"
Enter
Set TypingSpeed 40ms
Type "aigent --about"
Enter
Sleep 4s

# --- Print help ---
Ctrl+L
Set TypingSpeed 0ms
Type "# Print tool help"
Enter
Set TypingSpeed 40ms
Type "aigent --help | head -20"
Enter
Sleep 5s

# --- 1. Create a skill from natural language ---
Ctrl+L
Set TypingSpeed 0ms
Type '# Create skill from natural language prompt'
Enter
Set TypingSpeed 40ms
Type 'aigent new "greet with friendly hello world" --name hello-world'
Enter
Sleep 2s

# --- Print command help ---
Ctrl+L
Set TypingSpeed 0ms
Type '# Print command help'
Enter
Set TypingSpeed 40ms
Type "aigent new --help"
Enter
Sleep 5s

# --- 2. Validate ---
Ctrl+L
Set TypingSpeed 0ms
Type '# Check standard compliance'
Enter
Set TypingSpeed 40ms
Type "aigent validate hello-world"
Enter
Sleep 2s

# --- 3. Format ---
Ctrl+L
Set TypingSpeed 0ms
Type '# Check SKILL.md formatting'
Enter
Set TypingSpeed 40ms
Type "aigent format --check hello-world"
Enter
Sleep 2s

# --- 4. Score ---
Ctrl+L
Set TypingSpeed 0ms
Type '# Score against best-practices checklist'
Enter
Set TypingSpeed 40ms
Type "aigent score hello-world"
Enter
Sleep 5s

# --- 5. Probe activation ---
Ctrl+L
Set TypingSpeed 0ms
Type '# Probe activation against a query'
Enter
Set TypingSpeed 40ms
Type 'aigent probe hello-world --query "say hello"'
Enter
Sleep 5s

# --- 6. Check (validate + semantic) ---
Ctrl+L
Set TypingSpeed 0ms
Type '# Run semantic check'
Enter
Set TypingSpeed 40ms
Type "aigent check hello-world"
Enter
Sleep 2s

# --- 7. Generate prompt ---
Ctrl+L
Set TypingSpeed 0ms
Type '# Generate skill prompt'
Enter
Set TypingSpeed 40ms
Type "aigent prompt hello-world"
Enter
Sleep 5s

# --- Links ---                                                                                                                                     
Ctrl+L
Set TypingSpeed 0
Type "# References"
Enter
Enter
Type "# — Sources:   https://github.com/wkusnierczyk/aigent"
Enter
Type "# — Releases:  https://github.com/wkusnierczyk/aigent/releases"
Enter
Type "# — Crates:    https://crates.io/crates/aigent"
Enter
Type "# — Docs:      https://docs.rs/aigent"
Enter
Sleep 5s