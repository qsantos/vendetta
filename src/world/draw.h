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

#ifndef W_DRAW_H
#define W_DRAW_H

#include "world.h"
#include "../graphics.h"

void draw_event     (graphics_t* g, assets_t* a, character_t* player, event_t* e);
void draw_projectile(graphics_t* g, assets_t* a, character_t* player, projectile_t* p);
void draw_character (graphics_t* g, assets_t* a, character_t* player, character_t* c);
void draw_mine      (graphics_t* g, assets_t* a, character_t* player, mine_t* m);
void draw_building  (graphics_t* g, assets_t* a, character_t* player, building_t* b);
void draw_chunk     (graphics_t* g, assets_t* a, character_t* player, chunk_t* c, int step);
void draw_chunks    (graphics_t* g, assets_t* a, character_t* player, world_t* w, int step);
void draw_world     (graphics_t* g, assets_t* a, character_t* player, world_t* w, int step);

#endif
