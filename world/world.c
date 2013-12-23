#include "world.h"

#include <stdlib.h>
#include <string.h>
#include <limits.h>

#include "../util.h"

world_t* world_init(universe_t* u)
{
	world_t* w = CALLOC(world_t, 1);

	w->universe = u;
	character_init(&w->characters[0], u);
	for (size_t i = 0; i < 10; i++)
	{
		mine_t* m = &w->mines[i];
		int type = rand() % u->n_mines;
		mine_init(m, &u->mines[type]);
		m->o.x = (1 - 2*((float) rand()/INT_MAX)) * 500;
		m->o.y = (1 - 2*((float) rand()/INT_MAX)) * 500;
	}

	w->n_buildings = 0;
	w->a_buildings = 0;
	w->buildings = NULL;

	return w;
}

void world_exit(world_t* w)
{
	free(w);
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

	for (size_t i = 0; i < w->n_buildings; i++)
		if (object_isAt((object_t*) &w->buildings[i], x, y))
			return (object_t*) &w->buildings[i];

	return NULL;
}

void world_addBuilding(world_t* w, kindOf_building_t* b, float x, float y)
{
	if (w->n_buildings == w->a_buildings)
	{
		w->a_buildings = w->a_buildings ? 2*w->a_buildings : 1;
		w->buildings = CREALLOC(w->buildings, building_t, w->a_buildings);
	}

	building_init(&w->buildings[w->n_buildings++], b, x, y);
}
