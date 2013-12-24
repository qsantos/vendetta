#ifndef W_WORLD_H
#define W_WORLD_H

typedef struct world world_t;

#include "../universe/universe.h"
#include "character.h"
#include "mine.h"
#include "object.h"
#include "building.h"

struct world
{
	universe_t* universe;

	int n_characters;
	character_t characters[1];

	int n_mines;
	mine_t mines[10];

	int n_buildings;
	int a_buildings;
	building_t** buildings;
};

world_t* world_init(universe_t* u);
void     world_exit(world_t* w);

void world_doRound(world_t* w, float duration);

object_t* world_objectAt(world_t* w, float x, float y);

void world_addBuilding(world_t* w, kindOf_building_t* b, float x, float y);

#endif
