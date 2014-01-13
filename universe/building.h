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

#ifndef U_BUILDING_H
#define U_BUILDING_H

typedef struct kindOf_building kindOf_building_t;

#include "../graphics.h"
#include "transform.h"

struct kindOf_building
{
	char* name;

	float width;
	float height;
	int   sprite;
	int   n_sprites;

	int button_sprite;
	int button_index;

	// requisite materials
	transform_t build;

	// available material
	transform_t make;

	// available items
	size_t n_items;
	transform_t* items;
};

void kindOf_building_init(kindOf_building_t* b);
void kindOf_building_exit(kindOf_building_t* b);

void kindOf_building_sprite(kindOf_building_t* b, graphics_t* g, char* filename, int n_sprites);
void kindOf_building_button(kindOf_building_t* b, graphics_t* g, char* filename, int idx);

int kindOf_building_newItem(kindOf_building_t* b);

#endif
