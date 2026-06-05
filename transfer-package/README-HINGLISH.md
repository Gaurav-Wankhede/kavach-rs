# Kavach Tester Setup Guide — Windows Surface Pro 8

## Yeh Package Kya Hai?

Yeh package Claude Code ke saath Kavach enforcement system setup karne ke liye hai. Tester role ke liye configured hai — read-only access, no code modifications allowed.

## Package Contents

```
transfer-package/
├── CLAUDE.md                 # Global Engineering Directives (installed to ~/.claude/CLAUDE.md)
├── CLAUDE.legacy.md          # Prior bilingual/tester-role instructions (reference)
├── settings.json             # Claude Code settings (Windows paths)
├── setup-kavach-tester.ps1   # Automated setup script
├── README-HINGLISH.md        # Yeh file
├── rules/
│   ├── 00-core.md            # Session protocol
│   ├── 01-behavior.md        # Research-first behavior
│   ├── 04-anti-patterns.md   # Bug patterns to detect
│   ├── 06-pipeline.md        # Execution flow
│   └── 11-output.md          # Response format
├── agents/
│   ├── aegis-guardian.md     # Testing/verification agent
│   └── research-director.md  # Research agent
└── commands/
    ├── verify.md             # /verify command
    ├── bug-bounty.md         # /bug-bounty command
    ├── data.md               # /data command
    ├── arch.md               # /arch command
    ├── evidence-chain.md     # /evidence-chain command
    └── error.md              # /error command
```

## Installation Steps

### Step 1: Prerequisites Install Karo

PowerShell as Administrator open karo aur run karo:

```powershell
# Rust install karo
winget install Rustlang.Rustup

# Visual Studio Build Tools install karo
winget install Microsoft.VisualStudio.2022.BuildTools

# Git install karo
winget install Git.Git
```

### Step 2: Terminal Restart Karo

PATH refresh hone ke liye terminal close karke phir se open karo.

### Step 3: Kavach Build Karo

```powershell
# Repository clone karo
git clone <REPO_URL> C:\kavach-rs
cd C:\kavach-rs

# Build karo
cargo build --release

# Binary install karo
mkdir $env:LOCALAPPDATA\kavach
copy target\release\kavach.exe $env:LOCALAPPDATA\kavach\

# PATH mein add karo
[Environment]::SetEnvironmentVariable("Path", "$env:Path;$env:LOCALAPPDATA\kavach", "User")
```

### Step 4: Config Files Copy Karo

```powershell
# Directories create karo
mkdir $env:USERPROFILE\.claude\rules
mkdir $env:USERPROFILE\.claude\agents
mkdir $env:USERPROFILE\.claude\commands

# Files copy karo (transfer-package folder se)
copy transfer-package\CLAUDE.md $env:USERPROFILE\.claude\
copy transfer-package\settings.json $env:USERPROFILE\.claude\
copy transfer-package\rules\* $env:USERPROFILE\.claude\rules\
copy transfer-package\agents\* $env:USERPROFILE\.claude\agents\
copy transfer-package\commands\* $env:USERPROFILE\.claude\commands\
```

### Step 5: Database Initialize Karo

```powershell
kavach db init
kavach status
```

### Step 6: Claude Code Install Karo

```powershell
winget install Anthropic.ClaudeCode
```

## Ya Phir: Automated Setup Use Karo

Sab kuch ek script se:

```powershell
# PowerShell as Administrator
cd C:\kavach-rs\transfer-package
.\setup-kavach-tester.ps1 -RepoUrl "https://github.com/your-org/kavach-rs.git"
```

## Tester Permissions

| Allowed | Not Allowed |
|---------|-------------|
| Read files | Write/Edit files |
| Run tests | Git push/commit |
| Search code | Database modify |
| Git status/diff | Delete files |
| Database SELECT | Cargo build --release |

## Available Commands

| Command | Kya Karta Hai |
|---------|---------------|
| `/verify` | Tests aur validation check karo |
| `/bug-bounty` | Bugs dhundho code mein |
| `/data` | Database queries analyze karo |
| `/arch` | Architecture review karo |
| `/evidence-chain` | Research verify karo |
| `/error` | Error handling check karo |

## Roles Support

| Role | Commands |
|------|----------|
| Automation Testing | `/verify`, `/bug-bounty` |
| Manual Testing | `/verify` |
| Database Management | `/data` |
| Data Analysis | `/data`, `/arch` |
| Marketing | SEO plugin enabled |
| Research Analyst | `/evidence-chain` |

## Hinglish Mode

Claude responses mein Hindi + English mix use karega:

**Example:**
```
User: "Yeh test kyun fail ho raha hai?"

Claude: "Test fail ho raha hai kyunki expected value 200 thi 
lekin actual 404 mili. API endpoint `/users` exist nahi karta. 
Server running hai ya nahi check karo."
```

## Troubleshooting

### "kavach command not found"

PATH refresh nahi hua. Terminal restart karo ya:
```powershell
$env:Path = "$env:LOCALAPPDATA\kavach;$env:Path"
```

### "cargo not found"

Rust install nahi hua properly. Run karo:
```powershell
rustup-init.exe
```

### "Build failed"

Visual Studio Build Tools missing hai:
```powershell
winget install Microsoft.VisualStudio.2022.BuildTools
```

### "Permission denied"

PowerShell as Administrator run karo.

## Support

Koi problem ho to developer se contact karo. Logs check karo:
```powershell
kavach status
kavach db query --project <project> --category research
```
