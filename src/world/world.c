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
	if (c == NULL)
		return NULL;

	for (size_t i = 0; i < c->n_mines; i++)
	{
		mine_t* m = c->mines[i];
		if (&m->o == ignore)
			continue;
		if (object_isAt(&m->o, x, y))
			return &m->o;
	}
	for (size_t i = 0; i < c->n_buildings; i++)
	{
		building_t* b = c->buildings[i];
		if (&b->o == ignore)
			continue;
		if (object_isAt(&b->o, x, y))
			return &b->o;
	}

	return NULL;
}

/*
Goes around the origin following an anti-clockwise direction. Returns
0 when it completed the cycle.
*/
static char cycle(int* _x, int* _y)
{
	int x = *_x;
	int y = *_y;

	if (x == 0 && y == 0)
	{
		return 0;
	}
	else if (x >= 0)
	{
		if (y >= 0)
		{
			if (x > y)
				y++;
			else
				x--;
		}
		else
		{
			if (-y > x)
				x++;
			else
			{
				y++;
				if (y == 0)
					return 0;
			}
		}
	}
	else
	{
		if (y >= 0)
		{
			if (y > -x)
				x--;
			else
				y--;
		}
		else
		{
			if (-x > -y)
				y--;
			else
				x++;
		}
	}

	*_y = y;
	*_x = x;
	return 1;
}

#define LOOKFOR(FNAME, TYPE, FILTER, ...) \
TYPE##_t* FNAME(world_t* w, float x, float y, __VA_ARGS__) \
{ \
	int i = (y + w->o.h/2)/TILE_SIZE; \
	int j = (x + w->o.w/2)/TILE_SIZE; \
	if (!(0 <= i && i < w->rows && 0 <= j && j < w->cols)) \
		return NULL; \
	int ch = w->chunks[0].rows; \
	int cw = w->chunks[0].cols; \
	i /= ch; \
	j /= cw; \
 \
	TYPE##_t* ret = NULL; \
	float min_d = -1; \
	float max_r = w->chunk_rows > w->chunk_cols ? w->chunk_rows : w->chunk_cols; \
	for (int radius = 0; radius < max_r; radius++) \
	{ \
		int di = radius; \
		int dj = 0; \
		do \
		{ \
			int ti = i + di; \
			int tj = j + dj; \
			if (!(0 <= ti && ti < w->chunk_rows && 0 <= tj && tj < w->chunk_cols)) \
				continue; \
			chunk_t* c = CHUNK(w, i+di, j+dj); \
			for (size_t i = 0; i < c->n_##TYPE##s; i++) \
			{ \
				TYPE##_t* obj = c->TYPE##s[i]; \
				{ FILTER; } \
				float d = object_distance(&obj->o, x, y); \
				if (d < min_d || min_d < 0) \
				{ \
					ret = obj; \
					min_d = d; \
				} \
			} \
		} \
		while (cycle(&di, &dj)); \
		if (ret == NULL) \
			continue; \
		if (1.4142*min_d <= (radius-1)*ch*TILE_SIZE) \
			break; \
	} \
	return ret; \
}

LOOKFOR(world_findMine,           mine,     if (obj->t != t)                          continue, kindOf_mine_t* t)
LOOKFOR(world_findBuilding,       building, if (obj->t != t)                          continue, kindOf_building_t* t)
LOOKFOR(world_findEnnemyBuilding, building, if (obj->owner == p->o.uuid)              continue, character_t* p)
LOOKFOR(world_findSale,           building, if (building_onSale(obj,is_item,id) <= 0) continue, char is_item, int id)

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

static char canBuild_aux(chunk_t* c, object_t* o);
char world_canMine(world_t* w, float x, float y)
{
	object_t o = {O_BUILDING, 0, x, y, 32, 32};
	if (!object_contains(&w->o, &o))
		return 0;

	float minx = o.x - o.w/2;
	float maxx = minx + o.w;
	float miny = o.y - o.h;
	float maxy = o.y;
	for (float x = minx; x <= maxx; x += TILE_SIZE)
		for (float y = miny; y <= maxy; y += TILE_SIZE)
		{
			char l = world_getLandXY(w, x, y)/16;
			if (l == 4 || l == 10)
				return 0;
		}

	if (!canBuild_aux(world_chunkXY(w, o.x-o.w/2, o.y-o.h), &o)) return 0;
	if (!canBuild_aux(world_chunkXY(w, o.x-o.w/2, o.y    ), &o)) return 0;
	if (!canBuild_aux(world_chunkXY(w, o.x+o.w/2, o.y-o.h), &o)) return 0;
	if (!canBuild_aux(world_chunkXY(w, o.x+o.w/2, o.y    ), &o)) return 0;

	return 1;
}
mine_t* world_addMine(world_t* w, float x, float y, kindOf_mine_t* t)
{
	if (!world_canMine(w, x, y))
		return NULL;

	mine_t* m = mine_new(&w->objects, -1);
	mine_init(m, w, t, x, y);

	object_t o = m->o;
	chunk_pushMine(world_chunkXY(w, o.x-o.w/2, o.y-o.h), m);
	chunk_pushMine(world_chunkXY(w, o.x-o.w/2, o.y    ), m);
	chunk_pushMine(world_chunkXY(w, o.x+o.w/2, o.y-o.h), m);
	chunk_pushMine(world_chunkXY(w, o.x+o.w/2, o.y    ), m);

	return m;
}

static char canBuild_aux(chunk_t* c, object_t* o)
{
	if (c == NULL)
		return 0;
	for (size_t i = 0; i < c->n_mines; i++)
		if (object_overlaps(&c->mines[i]->o, o))
			return 0;
	for (size_t i = 0; i < c->n_buildings; i++)
		if (object_overlaps(&c->buildings[i]->o, o))
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
		{
			char l = world_getLandXY(w, x, y)/16;
			if (l != 0)
				return 0;
		}

	if (!canBuild_aux(world_chunkXY(w, o.x-o.w/2, o.y-o.h), &o)) return 0;
	if (!canBuild_aux(world_chunkXY(w, o.x-o.w/2, o.y    ), &o)) return 0;
	if (!canBuild_aux(world_chunkXY(w, o.x+o.w/2, o.y-o.h), &o)) return 0;
	if (!canBuild_aux(world_chunkXY(w, o.x+o.w/2, o.y    ), &o)) return 0;

	return 1;
}

building_t* world_addBuilding(world_t* w, float x, float y, kindOf_building_t* t, character_t* c)
{
	if (!world_canBuild(w, x, y, t))
		return NULL;

	building_t* b = building_new(&w->objects, -1);
	building_init(b, w, t, c->o.uuid, x, y);

	object_t o = b->o;
	chunk_pushBuilding(world_chunkXY(w, o.x-o.w/2, o.y-o.h), b);
	chunk_pushBuilding(world_chunkXY(w, o.x-o.w/2, o.y    ), b);
	chunk_pushBuilding(world_chunkXY(w, o.x+o.w/2, o.y-o.h), b);
	chunk_pushBuilding(world_chunkXY(w, o.x+o.w/2, o.y    ), b);

	return b;
}

void world_delBuilding(world_t* w, building_t* b)
{
	object_t o = b->o;
	chunk_delBuilding(world_chunkXY(w, o.x-o.w/2, o.y-o.h), o.uuid);
	chunk_delBuilding(world_chunkXY(w, o.x-o.w/2, o.y    ), o.uuid);
	chunk_delBuilding(world_chunkXY(w, o.x+o.w/2, o.y-o.h), o.uuid);
	chunk_delBuilding(world_chunkXY(w, o.x+o.w/2, o.y    ), o.uuid);

	building_exit(b);
	pool_t* p = &w->objects;
	pool_del(p, &b->o);
}
