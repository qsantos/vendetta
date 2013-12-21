#include "world.h"

#include <stdlib.h>
#include <string.h>
#include <limits.h>

void world_init(world_t* w, universe_t* u)
{
	w->universe = u;
	character_init(&w->characters[0], u);
	for (size_t i = 0; i < 10; i++)
	{
		mine_t* m = &w->mines[i];
		int type = rand() % u->n_mines;
		mine_init(m, &u->mines[type]);
		m->o.x = (float) rand() / INT_MAX * 1000;
		m->o.y = (float) rand() / INT_MAX * 1000;
	}
}

void world_doRound(world_t* w, float duration)
{
	character_doRound(&w->characters[0], duration);
}

object_t* world_objectAt(world_t* w, float x, float y)
{
	for (size_t i = 0; i < 1; i++)
		if (object_isAt((object_t*) &w->characters[i], x, y))
			return (object_t*) &w->characters[i];

	for (size_t i = 0; i < 10; i++)
		if (object_isAt((object_t*) &w->mines[i], x, y))
			return (object_t*) &w->mines[i];

	return NULL;
}
