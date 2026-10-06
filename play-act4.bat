@echo off
rem Ashen Sanctum: jump straight to Act 4 with a ready-made level 34 hero (its own save).
rem Pick the hero by adding a class: play-act4.bat --reaper  (or --valkyrie, --berserker, --inventor, --vampire)
title Ashen Sanctum - Act 4
wsl -d Ubuntu -e bash -lc "cd /mnt/d/projects/AshenSanctum && bash scripts/dev-build.sh run --act4 %*"
if errorlevel 1 pause
