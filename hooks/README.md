Repository hooks

This directory contains local Git hook scripts you can install into `.git/hooks` to
enforce repository policies on your local machine.

pushes. Install it locally with one of the following commands (they copy the
hook into your `.git/hooks` directory):

Windows (PowerShell):

```powershell
powershell -File hooks/install-hooks.ps1 -Enable
```
To remove the installed hooks:

Windows:

```powershell
powershell -File hooks/install-hooks.ps1 -Disable
```


Notes
- These files live in the repository and are intended for local install. They
  are not automatically active just by being in the repo.
- We will not push any install changes to the remote without project approval.
