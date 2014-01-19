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

#ifndef DRAW_H
#define DRAW_H

#include "world.h"
#include "../graphics.h"

void draw_character(graphics_t* g, character_t* p, character_t* c);
void draw_mine     (graphics_t* g, character_t* p, mine_t* m);
void draw_building (graphics_t* g, character_t* p, building_t* b);
void draw_world    (graphics_t* g, character_t* p, world_t* w);

#endif
