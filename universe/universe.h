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

#ifndef U_UNIVERSE_H
#define U_UNIVERSE_H

typedef struct universe universe_t;

#include "material.h"
#include "item.h"
#include "mine.h"
#include "building.h"
#include "skill.h"
#include "category.h"
#include "equipment.h"
#include "../graphics.h"

struct universe
{
	size_t n_materials;
	kindOf_material_t* materials;

	size_t n_items;
	kindOf_item_t* items;

	size_t n_mines;
	kindOf_mine_t* mines;

	size_t n_buildings;
	kindOf_building_t* buildings;

	kindOf_skill_t sskills[N_SPECIAL_SKILLS];

	size_t n_iskills;
	kindOf_skill_t* iskills;

	kindOf_status_t statuses[N_STATUSES];

	size_t n_categories;
	kindOf_category_t* categories;

	size_t n_slots;
	kindOf_slot_t* slots;

	// temporarily stores the harvest, transformation
	// and fabrication information
	transform_t* tmp_materials;
	transform_t* tmp_items;

};

universe_t* universe_init(graphics_t* g);
void        universe_exit(universe_t* u);

void universe_parse(universe_t* u, graphics_t* g, const char* filename);

#endif
