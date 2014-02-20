/*\
 *  Role playing, management and strategy game
 *  Copyright (C) 2013-2014 Quentin SANTOS
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with this program.  If not, see <http://www.gnu.org/licenses/>.
\*/

#ifndef GAME_H
#define GAME_H

typedef struct game game_t;

#include "settings.h"
#include "graphics.h"
#include "assets.h"
#include "universe/universe.h"
#include "world/world.h"
#include "overlay/overlay.h"

struct game
{
	settings_t* s;
	graphics_t* g;
	assets_t*   a;

	overlay_t*  o;
	universe_t* u;
	world_t*    w;

	float fps;

	character_t* player;
	char         autoEat[N_STATUSES];
};

void game_init(game_t* g, settings_t* s, graphics_t* gr, assets_t* a, char load);
void game_exit(game_t* g);
void game_loop(game_t* g);

void game_save(game_t* g, FILE* f);
void game_load(game_t* g, FILE* f);

#endif
