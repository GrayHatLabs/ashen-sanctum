#!/bin/bash
# PortMaster launcher for ASHEN SANCTUM.
# Copy this file and the "ashensanctum" folder into your ports directory
# (e.g. /roms/ports/ or /mnt/sdcard/ports/ depending on firmware).

XDG_DATA_HOME=${XDG_DATA_HOME:-$HOME/.local/share}

if [ -d "/opt/system/Tools/PortMaster/" ]; then
  controlfolder="/opt/system/Tools/PortMaster"
elif [ -d "/opt/tools/PortMaster/" ]; then
  controlfolder="/opt/tools/PortMaster"
elif [ -d "$XDG_DATA_HOME/PortMaster/" ]; then
  controlfolder="$XDG_DATA_HOME/PortMaster"
else
  controlfolder="/roms/ports/PortMaster"
fi

source $controlfolder/control.txt
[ -f "${controlfolder}/mod_${CFW_NAME}.txt" ] && source "${controlfolder}/mod_${CFW_NAME}.txt"
get_controls

GAMEDIR="/$directory/ports/ashensanctum"
cd "$GAMEDIR"
> "$GAMEDIR/log.txt" && exec > >(tee "$GAMEDIR/log.txt") 2>&1

export SDL_GAMECONTROLLERCONFIG="$sdl_controllerconfig"
chmod +x "$GAMEDIR/ashensanctum"

# gptokeyb only provides the SELECT+START exit hotkey; the game reads the pad natively.
$GPTOKEYB "ashensanctum" -c "$GAMEDIR/ashensanctum.gptk" &
pm_platform_helper "$GAMEDIR/ashensanctum"
./ashensanctum --fullscreen

pm_finish
