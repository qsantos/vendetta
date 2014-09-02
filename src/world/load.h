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

#ifndef W_LOAD_H
#define W_LOAD_H

#include "../cfg.h"
#include "world.h"

void load_listb    (cfg_t* cfg, char*  l, size_t n);
void load_listi    (cfg_t* cfg, int*   l, size_t n);
void load_listf_dic(cfg_t* cfg, float* l, size_t n);
void load_object   (cfg_t* cfg, object_t*    o);
void load_inventory(cfg_t* cfg, inventory_t* inv, universe_t* u);
void load_ai_data  (cfg_t* cfg, ai_data_t*   d);
void load_character(cfg_t* cfg, character_t* c);
void load_building (cfg_t* cfg, building_t*  b);
void load_world    (cfg_t* cfg, world_t*     w);

#endif
