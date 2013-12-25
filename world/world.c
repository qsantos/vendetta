#include "world.h"

#include <stdlib.h>
#include <string.h>
#include <limits.h>

#include "../util.h"

world_t* world_init(universe_t* u)
{
	world_t* w = CALLOC(world_t, 1);

	w->universe = u;

	w->n_characters = 1;
	character_init(&w->characters[0], u);

	w->n_mines = 10;
	for (int i = 0; i < w->n_mines; i++)
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
	for (int i = 0; i < w->n_characters; i++)
		character_exit(&w->characters[i]);

	for (int i = 0; i < w->n_mines; i++)
		mine_exit(&w->mines[i]);

	for (int i = 0; i < w->n_buildings; i++)
	{
		building_exit(w->buildings[i]);
		free(w->buildings[i]);
	}
	free(w->buildings);
	free(w);
}

void world_doRound(world_t* w, float duration)
{
	character_doRound(&w->characters[0], duration);
}

object_t* world_objectAt(world_t* w, float x, float y)
{
	for (int i = 0; i < w->n_characters; i++)
		if (object_isAt((object_t*) &w->characters[i], x, y))
			return (object_t*) &w->characters[i];

	for (int i = 0; i < w->n_mines; i++)
		if (object_isAt((object_t*) &w->mines[i], x, y))
			return (object_t*) &w->mines[i];

	for (int i = 0; i < w->n_buildings; i++)
		if (object_isAt((object_t*) w->buildings[i], x, y))
			return (object_t*) w->buildings[i];

	return NULL;
}

char world_canBeBuilt(world_t* w, float x, float y, float wi, float he)
{
	for (int i = 0; i < w->n_mines; i++)
		if (object_rect((object_t*) &w->mines[i], x, y, wi, he))
			return 0;

	for (int i = 0; i < w->n_buildings; i++)
		if (object_rect((object_t*) w->buildings[i], x, y, wi, he))
			return 0;

	return 1;
}

char world_canBuild(world_t* w, character_t* c, kindOf_building_t* b, float x, float y)
{
	return components_check(&b->build_req, &c->inventory) &&
	       world_canBeBuilt(w, x, y, b->width, b->height);
}

void world_addBuilding(world_t* w, kindOf_building_t* t, float x, float y)
{
	if (w->n_buildings == w->a_buildings)
	{
		w->a_buildings = w->a_buildings ? 2*w->a_buildings : 1;
		w->buildings = CREALLOC(w->buildings, building_t*, w->a_buildings);
	}

	building_t* b = CALLOC(building_t, 1);
	w->buildings[w->n_buildings++] = b;
	building_init(b, t, x, y);
}
