#ifndef W_CHARACTER_H
#define W_CHARACTER_H

typedef struct character character_t;

#include "object.h"
#include "inventory.h"
#include "building.h"
#include "skill.h"
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

	universe_t* universe;
	inventory_t inventory;
	building_t* inBuilding;

	skill_t  sskills[N_SPECIAL_SKILLS];
	skill_t* mskills;
	skill_t* iskills;
};

void character_init(character_t* c, universe_t* u);
void character_exit(character_t* c);

void character_workAt (character_t* c, object_t* o, float duration);
void character_doRound(character_t* c, float duration);

#endif
