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

#ifndef O_OVERLAY_H
#define O_OVERLAY_H

typedef struct overlay overlay_t;

#include "swbuilding.h"
#include "switems.h"
#include "swmaterials.h"
#include "swskills.h"
#include "swequipment.h"
#include "build.h"

struct overlay
{
	swbuilding_t  swbuilding;
	switems_t     switems;
	swmaterials_t swmaterials;
	swskills_t    swskills;
	swequipment_t swequipment;
	ov_build_t    build;

	size_t n_subwindows;
	subwindow_t** sw;

	sfVector2f mouse;
	int selected;
	float lastx;
	float lasty;

	char* msg;
	float msgDelay;
};

#include "../game.h"

void overlay_init(overlay_t* o, game_t* g);
void overlay_exit(overlay_t* o);

int  overlay_draw  (game_t* g, char do_draw);
int  overlay_cursor(game_t* g);
int  overlay_catch (game_t* g, int t);
int  overlay_wheel (game_t* g, int d);
void overlay_move  (game_t* g);

void overlay_doRound(game_t* g, float duration);

void overlay_message(game_t* g, const char* msg, float delay);

sfVector2f overlay_mouse(game_t* g);
sfVector2f   world_mouse(game_t* g);

#endif
