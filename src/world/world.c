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

#include "../mem.h"
#include "../rand.h"
#include "../voronoi/lloyd.h"

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

	w->n_characters = 0;
	w->characters = NULL;

	w->n_buildings = 0;
	w->a_buildings = 0;
	w->buildings = NULL;
}

void world_exit(world_t* w)
{
	for (size_t i = 0; i < w->n_buildings; i++)
	{
		building_exit(w->buildings[i]);
		free(w->buildings[i]);
	}
	free(w->buildings);

	for (size_t i = 0; i < w->n_characters; i++)
		character_exit(&w->characters[i]);
	free(w->characters);

	for (size_t i = 0; i < w->n_chunks; i++)
		chunk_exit(&w->chunks[i]);
	free(w->chunks);

	evtList_exit(&w->events);
}

void world_genmap(world_t* w, unsigned int seed)
{
	universe_t* u = w->universe;
	srand(seed);

	if (w->settings->verbosity >= 1)
		fprintf(stderr, "Proceeding to land generation\n");
	int cw = 64;
	int ch = 64;
	w->chunk_cols = ceil((float)w->settings->map_width  / cw);
	w->chunk_rows = ceil((float)w->settings->map_height / ch);
	w->n_chunks = w->chunk_cols*w->chunk_rows;
	w->chunks = CALLOC(chunk_t, w->n_chunks);
	for (int i = 0; i < w->chunk_rows; i++)
		for (int j = 0; j < w->chunk_cols; j++)
		{
			chunk_t* c = CHUNK(w, i, j);
			float x = TILE_SIZE*cw*(i-w->chunk_cols/2+.5);
			float y = TILE_SIZE*ch*(j-w->chunk_rows/2+.5+.5);
			chunk_init(c, x, y, cw, ch);
		}
	if (w->settings->verbosity >= 1)
		fprintf(stderr, "Land generation done\n");

	w->cols = w->chunk_cols * cw;
	w->rows = w->chunk_rows * ch;

	w->o.t = O_WORLD;
	w->o.w = w->cols * TILE_SIZE;
	w->o.h = w->rows * TILE_SIZE;
	w->o.x = 0;
	w->o.y = w->o.h/2;

	// BEGIN land generation
	// generate Voronoi diagram
	if (w->settings->verbosity >= 3)
		fprintf(stderr, "Initialiazing Voronoi diagram\n");
	vr_diagram_t v;
	vr_diagram_init(&v, w->cols, w->rows);
	size_t n_vrPoints = w->cols * w->rows / 50;
	for (size_t i = 0; i < n_vrPoints; i++)
	{
		double x = frnd(0, w->cols);
		double y = frnd(0, w->rows);
		vr_diagram_point(&v, (point_t){x,y});
	}
	for (int i = 1; i <= 2; i++)
	{
		if (w->settings->verbosity >= 3)
			fprintf(stderr, "Lloyd relexation pass %i\n", i);
		vr_lloyd_relaxation(&v);
	}
	vr_diagram_end(&v);
	if (w->settings->verbosity >= 3)
		fprintf(stderr, "Finished Voronoi generation\n");

	// assign land types to Voronoi regions
	static const float land_probas[] = {0.8, 0.05, 0.05, 0.04, 0.01, 0,0,0,0,0,0.05};
	short region_types[v.n_regions];
	for (size_t i = 0; i < v.n_regions; i++)
		region_types[i] = 16 * rnd_pick(land_probas);

	// rasterise map
	// TODO: clean that thing
	// yes, I know, this is the single most ugly and horrible
	// batch of code of this project; however, getting pixels
	// from a polygon is not totally trivial; to be cleaned later
	for (size_t i = 0; i < v.n_regions; i++)
	{
		short t = region_types[i];

		// set tiles in the i-th Voronoi region to proper type

		// first, get vertical bounds on the region
		vr_region_t* r = v.regions[i];
		int minj = w->cols;
		int maxj = 0;
		for (size_t k = 0; k < r->n_edges; k++)
		{
			segment_t* s = &r->edges[k]->s;
			if (s->a->y < minj) minj = floor(s->a->y);
			if (s->a->y > maxj) maxj = floor(s->a->y);
			if (s->b->y < minj) minj = floor(s->b->y);
			if (s->b->y > maxj) maxj = floor(s->b->y);
		}
		if (maxj >= w->cols)
			maxj = w->cols - 1;

		// then, consider each so-selected slide row
		for (int j = minj; j <= maxj; j++)
		{
			// find the portion of the row in the region
			int mini = w->rows;
			int maxi = 0;
			point_t a = {mini, j};
			point_t b = {maxi, j};
			segment_t s = {&a, &b};
			for (size_t k = 0; k < r->n_edges; k++)
			{
				point_t p;
				if (!segment_intersect(&p, &s, &r->edges[k]->s))
					continue;
				if (p.x < mini)
					mini = floor(p.x);
				if (p.x > maxi)
					maxi = floor(p.x);
			}
			if (maxi >= w->rows)
				maxi = w->rows - 1;

			// set this portion to proper type
			for (int i = mini; i <= maxi; i++)
				world_setLandIJ(w, i, j, t);
		}
	}

	vr_diagram_exit(&v);
	if (w->settings->verbosity >= 3)
		fprintf(stderr, "Rasterization done: lands generated\n");
	// END land generation

	// BEGIN region borders
#define LAND_TYPE(I,J) (world_getLandIJ(w,I,J) / 16)
#define LAND_SAME(I,J) ( \
	((I) < 0 || (I) >= w->rows || (J) < 0 || (J) >= w->cols) ? \
	1 : \
	t == LAND_TYPE(I,J) \
)
	static const char type2tile[16] = {0,5,2,13,4,7,12,8,3,15,6,11,14,9,10,1};
	for (int i = 0; i < w->rows; i++)
	for (int j = 0; j < w->cols; j++)
	{
		int t = LAND_TYPE(i,j);
		if (t == 0)
			continue;
		char top    = LAND_SAME(i,j-1);
		char right  = LAND_SAME(i+1,j);
		char bottom = LAND_SAME(i,j+1);
		char left   = LAND_SAME(i-1,j);
		int neighbor = (top<<3) | (right<<2) | (bottom<<1) | (left<<0);
		world_setLandIJ(w, i, j, 16*t + type2tile[neighbor]);
	}
	if (w->settings->verbosity >= 3)
		fprintf(stderr, "Fixed region borders\n");
	// END region borders

	for (size_t i = 0; i < w->n_chunks; i++)
		chunk_update(&w->chunks[i]);
	if (w->settings->verbosity >= 3)
		fprintf(stderr, "Chunk generated\n");

	evtList_init(&w->events);

	// BEGIN mine generation
	size_t n_mines = w->o.w*w->o.h / 100000;
	if (n_mines < u->n_mines)
		n_mines = u->n_mines;
	static const float mine_probas[] = {0.22,0.22,0.20,0.10,0.08,0.06,0.06,0.06};
	for (size_t i = 0; i < u->n_mines; i++) // ensure there is at least one of each
		world_randMine(w, i);
	for (size_t i = u->n_mines; i < n_mines; i++)
		world_randMine(w, rnd_pick(mine_probas));
	// END mine generation
	if (w->settings->verbosity >= 3)
		fprintf(stderr, "Generated %u mines\n", (unsigned) n_mines);
}

void world_start(world_t* w)
{
	universe_t* u = w->universe;

	// BEGIN character generation
	w->n_characters = 1 + w->settings->bots_count;
	w->characters = CALLOC(character_t, w->n_characters);
	for (size_t i = 0; i < w->n_characters; i++)
	{
		character_t* c = &w->characters[i];
		int type = rand() % u->n_characters;
		character_init(c, &u->characters[type], u, w);
		character_setPosition(c, cfrnd(w->o.w-20), cfrnd(w->o.h-20));
	}
	if (w->settings->verbosity >= 3)
		fprintf(stderr, "Generated %u characters\n", (unsigned) w->n_characters);
	// END character generation
}

static void save_object(object_t* o, FILE* f)
{
	fprintf(f, "%f %f %f %f", o->x, o->y, o->w, o->h);
}
void world_save(world_t* w, FILE* f)
{
	// seed
	fprintf(f, "seed = %#x\n", w->settings->seed);

	// characters
	fprintf(f, "%u characters\n", (unsigned) w->n_characters);
	for (size_t i = 0; i < w->n_characters; i++)
	{
		character_t* c = &w->characters[i];
		save_object(&c->o, f);
		fprintf(f, " %i %f %f\n", c->alive, c->go_x, c->go_y);
	}

	// buildings
	size_t n_buildings = 0;
	for (size_t i = 0; i < w->n_buildings; i++)
		if (w->buildings[i])
			n_buildings++;
	fprintf(f, "%u buildings\n", (unsigned) n_buildings);
	for (size_t i = 0; i < w->n_buildings; i++)
	{
		building_t* b = w->buildings[i];
		if (b == NULL)
			continue;
		save_object(&b->o, f);
		fprintf(f, " %f %f\n", b->build_progress, b->life);
	}
}

#define CLINE(...) do { \
	if (fscanf(f, __VA_ARGS__) < 0){ \
		fprintf(stderr, "Missing line in save\n"); \
		exit(1); \
	} \
	}while (0);
void world_load(world_t* w, FILE* f)
{
	universe_t* u = w->universe;

	// seed
	unsigned int seed;
	CLINE("seed = %x\n", &seed);
	world_genmap(w, seed);

	unsigned n_characters;
	CLINE("%u characters\n", &n_characters);
	w->n_characters = n_characters;
	w->characters = CALLOC(character_t, n_characters);
	for (size_t i = 0; i < n_characters; i++)
	{
		character_t* c = &w->characters[i];
		character_init(c, &u->characters[0], u, w);
		CLINE("%f %f %f %f %c %f %f\n", &c->o.x, &c->o.y, &c->o.w, &c->o.h, &c->alive, &c->go_x, &c->go_y);
	}

	return;
	unsigned n_buildings;
	CLINE("%u buildings\n", &n_buildings);
	w->n_buildings = n_buildings;
	w->a_buildings = n_buildings;
	w->buildings = CALLOC(building_t*, n_buildings);
	for (size_t i = 0; i < n_buildings; i++)
	{
		building_t* b = CALLOC(building_t, 1);
		building_init(b, NULL, NULL, 0, 0);
		CLINE("%f %f %f %f %f %f\n", &b->o.x, &b->o.y, &b->o.w, &b->o.h, &b->build_progress, &b->life);
	}
}

chunk_t* world_chunkXY(world_t* w, float x, float y)
{
	int i = (x + w->o.w/2)/TILE_SIZE;
	int j = (y + w->o.h/2)/TILE_SIZE;
	if (!(0 <= i && i < w->rows && 0 <= j && j < w->cols))
		return NULL;
	int ch = w->chunks[0].rows;
	int cw = w->chunks[0].cols;
	return CHUNK(w, i/ch, j/cw);
}

short* world_landXY(world_t* w, float x, float y)
{
	int i = (x + w->o.w/2)/TILE_SIZE;
	int j = (y + w->o.h/2)/TILE_SIZE;
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
	int cw = w->chunks[0].cols;
	int ch = w->chunks[0].rows;
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

void world_randMine(world_t* w, int type)
{
	mine_t m;
	mine_init(&m, &w->universe->mines[type]);
	while (1)
	{
		float x = cfrnd(w->o.w - 32);
		float y = cfrnd(w->o.h - 32) + 16;
		int t = world_getLandXY(w, x, y) / 16;
		if (t == 4 || t == 10)
			continue;

		chunk_t* c = world_chunkXY(w, x, y);
		m.o.x = x;
		m.o.y = y;
		chunk_pushMine(c, &m);
		break;
	}
}

void world_doRound(world_t* w, float duration)
{
	evtList_doRound(&w->events, duration);

	for (size_t i = 0; i < w->n_characters; i++)
		character_doRound(&w->characters[i], duration);
}

object_t* world_objectAt(world_t* w, float x, float y, object_t* ignore)
{
	for (size_t i = 0; i < w->n_characters; i++)
	{
		character_t* c = &w->characters[i];
		if (!c->alive)
			continue;
		if (&c->o == ignore)
			continue;
		if (object_isAt(&c->o, x, y))
			return &c->o;
	}

	chunk_t* c = world_chunkXY(w, x, y);
	if (c != NULL)
	for (size_t i = 0; i < c->n_mines; i++)
	{
		mine_t* m = &c->mines[i];
		if (&m->o == ignore)
			continue;
		if (object_isAt(&m->o, x, y))
			return &m->o;
	}

	for (size_t i = 0; i < w->n_buildings; i++)
	{
		building_t* b = w->buildings[i];
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
		mine_t* m = &c->mines[i];
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
	int i = (x + w->o.w/2)/TILE_SIZE;
	int j = (y + w->o.h/2)/TILE_SIZE;
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
	building_t* ret = NULL;
	float min_d = -1;
	for (size_t i = 0; i < w->n_buildings; i++)
	{
		building_t* m = w->buildings[i];
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

static char canBuild_aux(chunk_t* c, float x, float y, kindOf_building_t* t)
{
	object_t o = {O_BUILDING, x, y, t->width, t->height};
	if (c == NULL)
		return 0;
	for (size_t i = 0; i < c->n_mines; i++)
		if (object_overlaps(&c->mines[i].o, &o))
			return 0;
	return 1;
}
char world_canBuild(world_t* w, float x, float y, kindOf_building_t* t)
{
	object_t o = {O_BUILDING, x, y, t->width, t->height};
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

	if (!canBuild_aux(world_chunkXY(w, o.x-o.w/2, o.y-o.h), x, y, t)) return 0;
	if (!canBuild_aux(world_chunkXY(w, o.x-o.w/2, o.y    ), x, y, t)) return 0;
	if (!canBuild_aux(world_chunkXY(w, o.x+o.w/2, o.y-o.h), x, y, t)) return 0;
	if (!canBuild_aux(world_chunkXY(w, o.x+o.w/2, o.y    ), x, y, t)) return 0;

	for (size_t i = 0; i < w->n_buildings; i++)
	{
		building_t* b = w->buildings[i];
		if (b != NULL && object_overlaps(&b->o, &o))
			return 0;
	}

	return 1;
}

building_t* world_addBuilding(world_t* w, float x, float y, kindOf_building_t* t, character_t* c)
{
	if (w->n_buildings == w->a_buildings)
	{
		w->a_buildings = w->a_buildings ? 2*w->a_buildings : 1;
		w->buildings = CREALLOC(w->buildings, building_t*, w->a_buildings);
	}

	building_t* b = CALLOC(building_t, 1);
	w->buildings[w->n_buildings++] = b;
	building_init(b, t, c, x, y);
	return b;
}

void world_delBuilding(world_t* w, building_t* b)
{
	for (size_t i = 0; i < w->n_buildings; i++)
		if (w->buildings[i] == b)
			w->buildings[i] = NULL;
	building_exit(b);
	free(b);
}

character_t* world_findEnnemyCharacter(world_t* w, character_t* c)
{
	character_t* ret = NULL;
	float min_d = -1;
	for (size_t i = 0; i < w->n_characters; i++)
	{
		character_t* t = &w->characters[i];
		if (t == c || !t->alive)
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

building_t* world_findEnnemyBuilding (world_t* w, character_t* c)
{
	building_t* ret = NULL;
	float min_d = -1;
	for (size_t i = 0; i < w->n_buildings; i++)
	{
		building_t* t = w->buildings[i];
		if (t == NULL || t->owner == c)
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
