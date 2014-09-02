/*\
 *  Role playing, management and strategy game
 *  Copyright (C) 2013-2014 Quentin SANTOS
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with this program.  If not, see <http://www.gnu.org/licenses/>.
\*/

#include "world.h"

#include <stdlib.h>

void world_init(world_t* w, game_t* g)
{
	w->settings = g->s;
	w->universe = g->u;

	w->cols = 0;
	w->rows = 0;
	w->chunk_cols = 0;
	w->chunk_rows = 0;
	w->n_chunks = 0;
	w->chunks = NULL;

	pool_init(&w->objects);
}

void world_exit(world_t* w)
{
	pool_t* p = &w->objects;
	for (size_t i = 0; i < p->n_objects; i++)
	{
		object_t* o = pool_get(&w->objects, i);
		if (o == NULL)
			continue;

		if (o->t == O_CHARACTER)
			character_exit((character_t*) o);
		else if (o->t == O_MINE)
			mine_exit((mine_t*) o);
		else if (o->t == O_BUILDING)
			building_exit((building_t*) o);
	}
	pool_exit(p);

	for (size_t i = 0; i < w->n_chunks; i++)
		chunk_exit(&w->chunks[i]);
	free(w->chunks);

	evtList_exit(&w->events);
}

chunk_t* world_chunkXY(world_t* w, float x, float y)
{
	int i = (y + w->o.h/2)/TILE_SIZE;
	int j = (x + w->o.w/2)/TILE_SIZE;
	if (!(0 <= i && i < w->rows && 0 <= j && j < w->cols))
		return NULL;
	int ch = w->chunks[0].rows;
	int cw = w->chunks[0].cols;
	return CHUNK(w, i/ch, j/cw);
}

short* world_landXY(world_t* w, float x, float y)
{
	int i = (y + w->o.h/2)/TILE_SIZE;
	int j = (x + w->o.w/2)/TILE_SIZE;
	return world_landIJ(w, i, j);
}

short world_getLandXY(world_t* w, float x, float y)
{
	short* land = world_landXY(w, x, y);
	return land != NULL ? *land : 0;
}

void world_setLandXY(world_t* w, float x, float y, short l)
{
	short* land = world_landXY(w, x, y);
	if (land != NULL)
		*land = l;
}

short* world_landIJ(world_t* w, int i, int j)
{
	if (!(0 <= i && i < w->rows && 0 <= j && j < w->cols))
		return NULL;
	int ch = w->chunks[0].rows;
	int cw = w->chunks[0].cols;
	chunk_t* c = CHUNK(w, i/ch, j/cw);
	return &LAND(c, i%ch, j%cw);
}

short world_getLandIJ(world_t* w, int i, int j)
{
	short* land = world_landIJ(w, i, j);
	return land != NULL ? *land : 0;
}

void world_setLandIJ(world_t* w, int i, int j, short l)
{
	short* land = world_landIJ(w, i, j);
	if (land != NULL)
		*land = l;
}

void world_doRound(world_t* w, float duration)
{
	evtList_doRound(&w->events, duration);

	pool_t* p = &w->objects;
	for (size_t i = 0; i < p->n_objects; i++)
	{
		character_t* c = character_get(p, i);
		if (c != NULL)
			character_doRound(c, duration);
	}

	for (size_t i = 0; i < p->n_objects; i++)
	{
		projectile_t* q = projectile_get(p, i);
		if (q == NULL)
			continue;

		if (projectile_doRound(q, duration))
			continue;

		pool_del(p, &q->o);
	}
}

object_t* world_objectAt(world_t* w, float x, float y, object_t* ignore)
{
	pool_t* p = &w->objects;
	for (size_t i = 0; i < p->n_objects; i++)
	{
		character_t* c = character_get(p, i);
		if (c == NULL)
			continue;

		if (&c->o == ignore)
			continue;
		building_t* b = character_get_inBuilding(c);
		if (b != NULL || !c->alive)
			continue;
		if (object_isAt(&c->o, x, y))
			return &c->o;
	}

	chunk_t* c = world_chunkXY(w, x, y);
	if (c != NULL)
	for (size_t i = 0; i < c->n_mines; i++)
	{
		mine_t* m = c->mines[i];
		if (&m->o == ignore)
			continue;
		if (object_isAt(&m->o, x, y))
			return &m->o;
	}

	for (size_t i = 0; i < p->n_objects; i++)
	{
		building_t* b = building_get(p, i);
		if (b == NULL)
			continue;

		if (&b->o == ignore)
			continue;
		if (b != NULL && object_isAt(&b->o, x, y))
			return &b->o;
	}

	return NULL;
}

mine_t* findMine_chunk(chunk_t* c, float x, float y, kindOf_mine_t* t)
{
	mine_t* ret = NULL;
	float min_d = -1;
	for (size_t i = 0; i < c->n_mines; i++)
	{
		mine_t* m = c->mines[i];
		if (m->t != t)
			continue;
		float d = object_distance(&m->o, x, y);
		if (d < min_d || min_d < 0)
		{
			ret = m;
			min_d = d;
		}
	}
	return ret;
}
#define CHECK(I,J) do { \
	if (!(0 <= (I) && (I) < w->chunk_rows && 0 <= (J) && (J) < w->chunk_cols)) \
		continue; \
	mine_t* m = findMine_chunk(CHUNK(w, I, J), x, y, t); \
	if (m == NULL) \
		continue; \
	float d = object_distance(&m->o, x, y); \
	if (d < min_d || min_d < 0) \
	{ \
		ret = m; \
		min_d = d; \
	}\
} while(0)
mine_t* findMine_radius(world_t* w, float x, float y, kindOf_mine_t* t, int i, int j, int radius)
{
	mine_t* ret = NULL;
	float min_d = -1;
	for (int a = -radius; a <= radius; a++)
	{
		CHECK(i-radius, j+a);
		CHECK(i+radius, j+a);
		CHECK(i+a, j-radius);
		CHECK(i+a, j+radius);
	}
	return ret;
}
mine_t* world_findMine(world_t* w, float x, float y, kindOf_mine_t* t)
{
	int i = (y + w->o.h/2)/TILE_SIZE;
	int j = (x + w->o.w/2)/TILE_SIZE;
	if (!(0 <= i && i < w->rows && 0 <= j && j < w->cols))
		return NULL;
	int ch = w->chunks[0].rows;
	int cw = w->chunks[0].cols;
	i /= ch;
	j /= cw;

	mine_t* ret = NULL;
	float min_d = -1;
	for (int radius = 0; ret == NULL || 1.4142*min_d >= (radius-1)*ch*TILE_SIZE; radius++)
	{
		mine_t* m = findMine_radius(w, x, y, t, i, j, radius);
		if (m == NULL)
			continue;
		float d = object_distance(&m->o, x, y);
		if (d < min_d || min_d < 0)
		{
			ret = m;
			min_d = d;
		}
	}
	return ret;
}

building_t* world_findBuilding(world_t* w, float x, float y, kindOf_building_t* t)
{
	pool_t* p = &w->objects;
	building_t* ret = NULL;
	float min_d = -1;
	for (size_t i = 0; i < p->n_objects; i++)
	{
		building_t* m = building_get(p, i);
		if (m == NULL)
			continue;

		if (t == NULL || m->t == t)
		{
			float d = object_distance(&m->o, x, y);
			if (min_d < 0 || d < min_d)
			{
				ret = m;
				min_d = d;
			}
		}
	}
	return ret;
}

static char canBuild_aux(chunk_t* c, object_t* o)
{
	if (c == NULL)
		return 0;
	for (size_t i = 0; i < c->n_mines; i++)
		if (object_overlaps(&c->mines[i]->o, o))
			return 0;
	return 1;
}
char world_canBuild(world_t* w, float x, float y, kindOf_building_t* t)
{
	object_t o = {O_BUILDING, 0, x, y, t->width, t->height};
	if (!object_contains(&w->o, &o))
		return 0;

	float minx = o.x - o.w/2;
	float maxx = minx + o.w;
	float miny = o.y - o.h;
	float maxy = o.y;
	for (float x = minx; x <= maxx; x += TILE_SIZE)
		for (float y = miny; y <= maxy; y += TILE_SIZE)
			if (world_getLandXY(w, x, y)/16 != 0)
				return 0;

	if (!canBuild_aux(world_chunkXY(w, o.x-o.w/2, o.y-o.h), &o)) return 0;
	if (!canBuild_aux(world_chunkXY(w, o.x-o.w/2, o.y    ), &o)) return 0;
	if (!canBuild_aux(world_chunkXY(w, o.x+o.w/2, o.y-o.h), &o)) return 0;
	if (!canBuild_aux(world_chunkXY(w, o.x+o.w/2, o.y    ), &o)) return 0;

	pool_t* p = &w->objects;
	for (size_t i = 0; i < p->n_objects; i++)
	{
		building_t* b = building_get(p, i);
		if (b == NULL)
			continue;

		if (object_overlaps(&b->o, &o))
			return 0;
	}

	return 1;
}

building_t* world_addBuilding(world_t* w, float x, float y, kindOf_building_t* t, character_t* c)
{
	pool_t* p = &w->objects;
	building_t* b = building_new(p, -1);
	building_init(b, w, t, c->o.uuid, x, y);

	chunk_t* k = world_chunkXY(w, x, y);
	chunk_pushBuilding(k, b);

	return b;
}

void world_delBuilding(world_t* w, building_t* b)
{
	chunk_t* c = world_chunkXY(w, b->o.x, b->o.y);
	chunk_delBuilding(c, b->o.uuid);

	building_exit(b);
	pool_t* p = &w->objects;
	pool_del(p, &b->o);
}

character_t* world_findEnnemyCharacter(world_t* w, character_t* c)
{
	pool_t* p = &w->objects;

	character_t* ret = NULL;
	float min_d = -1;
	for (size_t i = 0; i < p->n_objects; i++)
	{
		character_t* t = character_get(p, i);
		if (t == NULL)
			continue;

		if (t == c)
			continue;
		building_t* b = character_get_inBuilding(t);
		if (b != NULL || !t->alive)
			continue;
		float d = object_distance(&c->o, t->o.x, t->o.y);
		if (min_d < 0 || d < min_d)
		{
			ret = t;
			min_d = d;
		}
	}
	return ret;
}

building_t* world_findEnnemyBuilding(world_t* w, character_t* c)
{
	pool_t* p = &w->objects;
	building_t* ret = NULL;
	float min_d = -1;
	for (size_t i = 0; i < p->n_objects; i++)
	{
		building_t* b = building_get(p, i);
		if (b == NULL)
			continue;

		if (b->owner == c->o.uuid)
			continue;

		float d = object_distance(&c->o, b->o.x, b->o.y);
		if (min_d < 0 || d < min_d)
		{
			ret = b;
			min_d = d;
		}
	}
	return ret;
}

building_t* world_findSale(world_t* w, character_t*c, char is_item, int id)
{
	pool_t* p = &w->objects;
	building_t* ret = NULL;
	float min_d = -1;
	for (size_t i = 0; i < p->n_objects; i++)
	{
		building_t* b = building_get(p, i);
		if (b == NULL)
			continue;

		if (building_onSale(b, is_item, id) <= 0)
			continue;

		float d = object_distance(&c->o, b->o.x, b->o.y);
		if (min_d < 0 || d < min_d)
		{
			ret = b;
			min_d = d;
		}
	}
	return ret;
}
