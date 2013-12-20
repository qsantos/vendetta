#include "world.h"

#include <stdlib.h>
#include <string.h>
#include <limits.h>

void world_init(world_t* w)
{
	memset(w, 0, sizeof(world_t));
	for (size_t i = 0; i < 10; i++)
	{
		w->mines[i].x = (float) rand() / INT_MAX * 1000;
		w->mines[i].y = (float) rand() / INT_MAX * 1000;
	}
}

void world_doRound(world_t* w, float duration)
{
	character_doRound(&w->characters[0], duration);
}
