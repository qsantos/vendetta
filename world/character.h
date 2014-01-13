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

#ifndef W_CHARACTER_H
#define W_CHARACTER_H

typedef struct character character_t;

#include "object.h"
#include "inventory.h"
#include "building.h"
#include "skill.h"
#include "status.h"
#include "world.h"
#include "../universe/universe.h"

typedef enum
{
	D_NORTH,
	D_EAST,
	D_SOUTH,
	D_WEST,
} direction_t;

struct character
{
	object_t o;

	float       go_x;
	float       go_y;
	object_t*   go_o;
	direction_t dir;
	float       step;

	char is_player;
	int  bot_step;

	universe_t* universe;
	world_t*    world;

	inventory_t inventory;
	building_t* inBuilding;

	skill_t  sskills[N_SPECIAL_SKILLS];
	skill_t* mskills;
	skill_t* iskills;

	status_t statuses[N_STATUSES];

	int* equipment;
};

void character_init(character_t* c, universe_t* u, world_t* w);
void character_exit(character_t* c);

float character_vitality(character_t* c);
void  character_weary   (character_t* c, float f);

void character_workAt  (character_t* c, object_t* o, float duration);
void character_doRound (character_t* c, float duration);

void character_setPosition(character_t* c, float x, float y);

#endif
