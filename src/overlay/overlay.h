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

	int selected;
	float lastx;
	float lasty;
};

#include "../game.h"

overlay_t* overlay_init(graphics_t* g);
void       overlay_exit(overlay_t* o);

void overlay_cursor(overlay_t* o, game_t* g);
int  overlay_draw  (overlay_t* o, game_t* g, char do_draw);

void overlay_move  (overlay_t* o, game_t* g, int x, int y);
int  overlay_catch (overlay_t* o, game_t* g, int x, int y, int t);
int  overlay_wheel (overlay_t* o, int x, int y, int d);

sfVector2f overlay_mouse(graphics_t* g);

#endif
