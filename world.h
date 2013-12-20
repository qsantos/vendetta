#ifndef WORLD_H
#define WORLD_H

#include "character.h"

typedef struct world world_t;

struct world
{
	character_t characters[1];
};

void world_init(world_t* w);

void world_doRound(world_t* w, float duration);

#endif
