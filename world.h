#ifndef WORLD_H
#define WORLD_H

#include "character.h"
#include "mine.h"

typedef struct world world_t;

struct world
{
	character_t characters[1];
	mine_t      mines[10];
};

void world_init(world_t* w);

void world_doRound(world_t* w, float duration);

#endif
