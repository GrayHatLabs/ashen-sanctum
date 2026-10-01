# Pull the latest generated art into the game: pack sheets in the art repo, embed them, rebuild.
# Usage (PowerShell): D:\projects\AshenSanctum\scripts\update-art.ps1
python D:\projects\AshenSanctum-art\tools\pack.py
if ($?) { python D:\projects\AshenSanctum\scripts\import_art.py }
if ($?) { wsl -d Ubuntu -e bash /mnt/d/projects/AshenSanctum/scripts/dev-build.sh }
