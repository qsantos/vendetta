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

#ifndef W_BUILDING_H
#define W_BUILDING_H

typedef struct building building_t;

#include "object.h"
#include "character.h"
#include "../universe/building.h"

struct building
{
	object_t o;

	kindOf_building_t* t;
	uuid_t owner;

	float build_progress;
	float life;

	size_t work_n;
	int*   work_list;
	float  work_progress;
};

void building_init(building_t* b, kindOf_building_t* t, character_t* owner, float x, float y);
void building_exit(building_t* b);

float building_build   (building_t* b, float work);
float building_attacked(building_t* b, float work, character_t* a);

void building_work_enqueue(building_t* b, int c);
void building_work_dequeue(building_t* b, size_t n);

#endif
