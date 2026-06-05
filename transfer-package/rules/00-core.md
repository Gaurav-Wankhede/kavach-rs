# Core — Tester Protocol (Hinglish)

Binary: `%LOCALAPPDATA%\kavach\kavach.exe`

## Session Start (MANDATORY)

```powershell
kavach status
kavach db kanban --project <slug>
```

Open items hain → pehle woh execute karo. Nahi hain → user ka prompt wait karo.

## CLI Reference

```powershell
kavach db kanban --project <slug>              # Open tasks dikhao
kavach db query --project <slug> --category <cat>
kavach db find-project --path "$PWD"
kavach session init | end | compact
kavach gates <gate> --verify <prompt>
```

## Tester Role

| Action | Allowed | Denied |
|--------|---------|--------|
| Read code | Yes | - |
| Run tests | Yes | - |
| Search/grep | Yes | - |
| Git status/diff | Yes | - |
| Database SELECT | Yes | - |
| Write/Edit files | - | No |
| Git push/commit | - | No |
| Database modify | - | No |
