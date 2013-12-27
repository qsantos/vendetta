#include "world.h"

#include <stdlib.h>
#include <string.h>
#include <limits.h>

#include "../util.h"

#define TILE_SIZE 32

world_t* world_init(universe_t* u)
{
	world_t* w = CALLOC(world_t, 1);
	w->universe = u;

	w->tilesx = 100;
	w->tilesy = 100;
	w->terrain = CALLOC(char, w->tilesx*w->tilesy);
	for (int i = 0; i < w->tilesx; i++)
		for (int j = 0; j < w->tilesy; j++)
			w->terrain[i*w->tilesx + j] = rand() % 17;

	float width  = w->tilesx * TILE_SIZE;
	float height = w->tilesy * TILE_SIZE;

	w->n_characters = 1;
	character_init(&w->characters[0], u);

	w->n_mines = w->tilesx*w->tilesy / 100;
	w->mines = CALLOC(mine_t, w->n_mines);
	for (int i = 0; i < w->n_mines; i++)
	{
		mine_t* m = &w->mines[i];
		int type = rand() % u->n_mines;
		mine_init(m, &u->mines[type]);
		m->o.x = (0.5 - ((float) rand()/INT_MAX)) * width;
		m->o.y = (0.5 - ((float) rand()/INT_MAX)) * height;
	}

	w->n_buildings = 0;
	w->a_buildings = 0;
	w->buildings = NULL;

	return w;
}

void world_exit(world_t* w)
{
	for (int i = 0; i < w->n_buildings; i++)
	{
		building_exit(w->buildings[i]);
		free(w->buildings[i]);
	}
	free(w->buildings);

	for (int i = 0; i < w->n_mines; i++)
		mine_exit(&w->mines[i]);

	for (int i = 0; i < w->n_characters; i++)
		character_exit(&w->characters[i]);

	free(w->terrain);
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

mine_t* world_findMine(world_t* w, float x, float y, kindOf_mine_t* t)
{
	mine_t* ret = NULL;
	float min_d = -1;
	for (int i = 0; i < w->n_mines; i++)
	{
		mine_t* m = &w->mines[i];
		if (t == NULL || m->t == t)
		{
			float d = object_distance((object_t*) m, x, y);
			if (min_d < 0 || d < min_d)
			{
				ret = m;
				min_d = d;
			}
		}
	}
	return ret;
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
	return transform_check(&b->build, &c->inventory) &&
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
