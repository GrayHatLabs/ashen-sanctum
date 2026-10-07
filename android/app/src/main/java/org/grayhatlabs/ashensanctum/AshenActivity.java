package org.grayhatlabs.ashensanctum;

import org.libsdl.app.SDLActivity;

/** Ashen Sanctum: SDL runs the game from libmain.so (SDL_main in src/main.rs). */
public class AshenActivity extends SDLActivity {
    @Override
    protected String[] getLibraries() {
        return new String[] { "SDL2", "main" };
    }
}
