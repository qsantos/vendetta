#ifndef W_WORLD_H
#define W_WORLD_H

#include "../universe/universe.h"
#include "character.h"
#include "mine.h"
#include "object.h"

typedef struct world world_t;

struct world
{
	universe_t* universe;

	character_t characters[1];
	mine_t      mines[10];
};

void world_init(world_t* w, universe_t* u);

void world_doRound(world_t* w, float duration);

object_t* world_objectAt(world_t* w, float x, float y);

#endif
