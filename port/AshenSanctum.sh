#!/bin/bash
# Launcher for ASHEN SANCTUM (PortMaster style, but also runs without PortMaster).
# Copy this file and the "ashensanctum" folder into your ports directory
# (e.g. /userdata/roms/ports on Knulli, /mnt/mmc/ROMS/Ports on muOS).

# The game folder sits next to this script, wherever the firmware keeps ports.
GAMEDIR="$(cd "$(dirname "$0")" && pwd)/ashensanctum"
cd "$GAMEDIR" || exit 1
# Log everything from the very first line (send log.txt if the game doesn't start).
exec > "$GAMEDIR/log.txt" 2>&1
echo "Ashen Sanctum launcher: $(date)"
echo "script: $0"
echo "gamedir: $GAMEDIR"
uname -a
ldd --version 2>&1 | head -1

XDG_DATA_HOME=${XDG_DATA_HOME:-$HOME/.local/share}
controlfolder=""
for d in /opt/system/Tools/PortMaster /opt/tools/PortMaster "$XDG_DATA_HOME/PortMaster" \
         /userdata/system/.local/share/PortMaster /userdata/roms/ports/PortMaster \
         /mnt/mmc/MUOS/PortMaster /roms/ports/PortMaster /storage/roms/ports/PortMaster; do
  if [ -f "$d/control.txt" ]; then
    controlfolder="$d"
    break
  fi
done
echo "PortMaster: ${controlfolder:-not found}"

if [ -n "$controlfolder" ]; then
  source "$controlfolder/control.txt"
  [ -f "${controlfolder}/mod_${CFW_NAME}.txt" ] && source "${controlfolder}/mod_${CFW_NAME}.txt"
  get_controls
  export SDL_GAMECONTROLLERCONFIG="$sdl_controllerconfig"
fi

chmod +x "$GAMEDIR/ashensanctum"
echo "libraries:"
ldd "$GAMEDIR/ashensanctum" 2>&1

# gptokeyb only provides the SELECT+START exit hotkey; the game reads the pad natively.
if [ -n "$controlfolder" ] && [ -n "$GPTOKEYB" ]; then
  $GPTOKEYB "ashensanctum" -c "$GAMEDIR/ashensanctum.gptk" &
  type pm_platform_helper >/dev/null 2>&1 && pm_platform_helper "$GAMEDIR/ashensanctum"
fi

echo "starting game"
./ashensanctum --fullscreen
echo "game exited with code $?"

if [ -n "$controlfolder" ]; then
  type pm_finish >/dev/null 2>&1 && pm_finish
fi
