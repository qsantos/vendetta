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
#include "../universe/character.h"
#include "../universe/material.h"
#include "../universe/mine.h"
#include "../universe/universe.h"
#include "../ai.h"

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

	kindOf_character_t* t;
	char alive;

	float  go_x;
	float  go_y;
	uuid_t go_o;

	direction_t dir;
	float       step;
	char        inWater;

	ai_t* ai;

	universe_t* universe;
	world_t*    world;

	inventory_t inventory;
	uuid_t hasBuilding;
	uuid_t inBuilding;

	skill_t* skills;
	status_t statuses[N_STATUSES];

	int* equipment;
};

void character_init(character_t* c, kindOf_character_t* t, universe_t* u, world_t* w);
void character_exit(character_t* c);

float character_vitality(character_t* c);
float character_getSkill(character_t* c, int skill);
float character_maxOfStatus  (character_t* c, int s);
float character_maxOfMaterial(character_t* c, kindOf_material_t* m);
float character_armor(character_t* c);

void character_addStatus(character_t* c, int s, float q);
void character_weary   (character_t* c, float f);
void character_train   (character_t* c, int skill, float work);
void character_workAt  (character_t* c, object_t* o, float duration);
void character_attack  (character_t* c, object_t* o);
void character_move    (character_t* c, float duration, float dx, float dy);
void character_doRound (character_t* c, float duration);

char character_eat   (character_t* c, int material);
void character_eatFor(character_t* c, int status);

void  character_setPosition(character_t* c, float x, float y);
float character_attacked   (character_t* c, float work, character_t* a);

void character_goMine   (character_t* c, kindOf_mine_t* t);
char character_buildAuto(character_t* c, kindOf_building_t* t);
char character_buildAt  (character_t* c, kindOf_building_t* t, float x, float y);
char character_delHome  (character_t* c);

building_t* character_get_inBuilding (character_t* c);
building_t* character_get_hasBuilding(character_t* c);

size_t character_currentAction(character_t* c, char* buffer, size_t n);

#endif
