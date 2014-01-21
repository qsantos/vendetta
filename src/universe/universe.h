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

#include "../ai.h"
#include "event.h"
#include "character.h"
#include "material.h"
#include "item.h"
#include "mine.h"
#include "building.h"
#include "skill.h"
#include "category.h"
#include "equipment.h"
#include "ini.h"

struct universe
{
	size_t n_bots;
	ai_t*  bots;

	size_t n_events;
	kindOf_event_t* events;

	size_t n_characters;
	kindOf_character_t* characters;

	size_t n_materials;
	kindOf_material_t* materials;

	size_t n_items;
	kindOf_item_t* items;

	size_t n_mines;
	kindOf_mine_t* mines;

	size_t n_buildings;
	kindOf_building_t* buildings;

	size_t n_skills;
	kindOf_skill_t* skills;

	// TODO: temporary
	size_t n_iskills;
	int* iskills;

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

#include "../game.h"

universe_t* universe_init(game_t* g);
void        universe_exit(universe_t* u);

void universe_init_events   (universe_t* u, graphics_t* g, cfg_group_t* gr);
void universe_init_materials(universe_t* u, graphics_t* g, cfg_group_t* gr);
void universe_init_mines    (universe_t* u, graphics_t* g, cfg_group_t* gr);
void universe_init_iskills  (universe_t* u, graphics_t* g, cfg_group_t* gr);
void universe_init_items    (universe_t* u, graphics_t* g, cfg_group_t* gr);
void universe_init_buildings(universe_t* u, graphics_t* g, cfg_group_t* gr);

#endif
