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
#include "universe/universe.h"
#include "world/world.h"
#include "overlay/overlay.h"

struct game
{
	graphics_t* g;
	overlay_t*  o;
	universe_t* u;
	world_t*    w;

	character_t* player;
	char         autoEat[N_STATUSES];
};

void game_init(game_t* g, settings_t* s);
void game_exit(game_t* g);
void game_loop(game_t* g);

#endif
