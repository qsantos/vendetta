#include "world.h"

#include <string.h>

void world_init(world_t* w)
{
	memset(w, 0, sizeof(world_t));
}

void world_doRound(world_t* w, float duration)
{
	character_doRound(&w->characters[0], duration);
}
