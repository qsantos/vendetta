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
#include <string.h>
#include <limits.h>
#include <math.h>

#include "../util.h"
#include "../voronoi/lloyd.h"

#define TILE_SIZE 32

world_t* world_init(universe_t* u)
{
	world_t* w = CALLOC(world_t, 1);
	w->universe = u;

	// BEGIN map generation
	w->tilesx = 100;
	w->tilesy = 100;
	w->terrain = CALLOC(char, w->tilesx*w->tilesy);
	/*
	for (int i = 0; i < w->tilesx; i++)
		for (int j = 0; j < w->tilesy; j++)
			w->terrain[i*w->tilesx + j] = rand() % 17;
	*/

	// generate Voronoi diagram
	vr_diagram_t v;
	vr_diagram_init(&v, w->tilesx, w->tilesy);
	for (size_t i = 0; i < 1000; i++)
	{
		double x = ( (double) rand() / INT_MAX ) * w->tilesx;
		double y = ( (double) rand() / INT_MAX ) * w->tilesy;
		vr_diagram_point(&v, (point_t){x,y});
	}
	vr_lloyd_relaxation(&v);
	vr_lloyd_relaxation(&v);
	vr_diagram_end(&v);

	// assign land types to Voronoi regions
	char region_types[v.n_regions];
	for (size_t i = 0; i < v.n_regions; i++)
		region_types[i] = 16 * (rand() % 6);

	// rasterise map
	// TODO: clean that thing
	// yes, I know, this is the single most ugly and horrible
	// batch of code of this project; however, getting pixels
	// from a polygon is not totally trivial; to be cleaned later
	for (size_t i = 0; i < v.n_regions; i++)
	{
		char t = region_types[i];

		// set tiles in the i-th Voronoi region to proper type

		// first, get vertical bounds on the region
		vr_region_t* r = v.regions[i];
		int miny = w->tilesy;
		int maxy = 0;
		for (size_t j = 0; j < r->n_edges; j++)
		{
			segment_t* s = &r->edges[j]->s;
			if (s->a->y < miny) miny = floor(s->a->y);
			if (s->a->y > maxy) maxy = floor(s->a->y);
			if (s->b->y < miny) miny = floor(s->b->y);
			if (s->b->y > maxy) maxy = floor(s->b->y);
		}
		if (maxy >= w->tilesy)
			maxy = w->tilesy - 1;

		// then, consider each so-selected slide row
		for (int y = miny; y <= maxy; y++)
		{
			// find the portion of the row in the region
			int rowmin = w->tilesx;
			int rowmax = 0;
			point_t a = {rowmin, y};
			point_t b = {rowmax, y};
			segment_t s = {&a, &b};
			for (size_t j = 0; j < r->n_edges; j++)
			{
				point_t p;
				if (!segment_intersect(&p, &s, &r->edges[j]->s))
					continue;
				if (p.x < rowmin)
					rowmin = floor(p.x);
				if (p.x > rowmax)
					rowmax = floor(p.x);
			}
			if (rowmax >= w->tilesx)
				rowmax = w->tilesx - 1;

			// set this portion to proper type
			for (int x = rowmin; x <= rowmax; x++)
				w->terrain[y*w->tilesx + x] = t;
		}
	}

	vr_diagram_exit(&v);
	// END map generation

	// BEGIN land type borders
#define LAND_TYPE(I,J) (w->terrain[(I)*w->tilesx + (J)] / 16)
#define LAND_SAME(I,J) ( \
	((I) < 0 || (I) >= w->tilesx || (J) < 0 || (J) >= w->tilesy) ? \
	1 : \
	t == LAND_TYPE(I,J) \
)
	static const char type2tile[16] = {0,5,2,13,4,7,12,8,3,15,6,11,14,9,10,1};
	for (int i = 0; i < w->tilesx; i++)
	for (int j = 0; j < w->tilesy; j++)
	{
		int t = LAND_TYPE(i,j);
		char top    = LAND_SAME(i,j-1);
		char right  = LAND_SAME(i+1,j);
		char bottom = LAND_SAME(i,j+1);
		char left   = LAND_SAME(i-1,j);
		int neighbor = (top<<3) | (right<<2) | (bottom<<1) | (left<<0);
		w->terrain[i*w->tilesx + j] += type2tile[neighbor];
	}
	// END land type borders

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
