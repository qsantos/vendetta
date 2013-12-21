#ifndef W_WORLD_H
#define W_WORLD_H

typedef struct world world_t;

#include "../universe/universe.h"
#include "character.h"
#include "mine.h"
#include "object.h"

struct world
{
	universe_t* universe;

	character_t characters[1];
	mine_t      mines[10];
};

world_t* world_init(universe_t* u);
void     world_exit(world_t* w);

void world_doRound(world_t* w, float duration);

object_t* world_objectAt(world_t* w, float x, float y);

#endif
