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

#include "world_gen.h"

#include <math.h>

#include "../mem.h"
#include "../rand.h"
#include "../voronoi/lloyd.h"

void world_genmap(world_t* w, unsigned int seed)
{
	universe_t* u = w->universe;
	srand(seed);

	if (w->settings->verbosity >= 1)
		fprintf(stderr, "Proceeding to land generation\n");

	int cw = 64;
	int ch = 64;
	w->chunk_cols = ceil(w->cols / cw);
	w->chunk_rows = ceil(w->rows / ch);
	w->n_chunks = w->chunk_cols*w->chunk_rows;
	w->chunks = CALLOC(chunk_t, w->n_chunks);
	for (int i = 0; i < w->chunk_rows; i++)
		for (int j = 0; j < w->chunk_cols; j++)
		{
			chunk_t* c = CHUNK(w, i, j);
			float x = TILE_SIZE*cw*(j-.5*w->chunk_cols+.5);
			float y = TILE_SIZE*ch*(i-.5*w->chunk_rows+.5+.5);
			chunk_init(c, x, y, cw, ch);
		}

	if (w->settings->verbosity >= 1)
		fprintf(stderr, "Prepared chunks\n");

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
	vr_diagram_init(&v, w->rows, w->cols);
	size_t n_vrPoints = w->rows * w->cols / 50;
	for (size_t i = 0; i < n_vrPoints; i++)
	{
		double i = frnd(0, w->rows);
		double j = frnd(0, w->cols);
		vr_diagram_point(&v, (point_t){i,j});
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
	static const float land_probas[] = {0, 0.25, 0.25, 0.2, 0.05, 0,0,0,0,0,0.25};

	short region_types[v.n_regions];
	for (size_t i = 0; i < v.n_regions; i++)
		region_types[i] = 0;

	size_t n_biomes = w->rows * w->cols / 1000;
	for (size_t k = 0; k < n_biomes; k++)
	{
		int t = rnd_pick(land_probas);
		float i = frnd(0, w->rows);
		float j = frnd(0, w->cols);
		float d = frnd(5, 10);
		d *= d;
		for (size_t k = 0; k < v.n_regions; k++)
		{
			vr_region_t* r = v.regions[k];
			point_t p = point_minus((point_t){i,j}, r->p);
			if (p.x*p.x + p.y*p.y < d)
				region_types[k] = t;
		}
	}

	// rasterise map
	// TODO: clean that thing
	// yes, I know, this is the single most ugly and horrible
	// batch of code of this project; however, getting pixels
	// from a polygon is not totally trivial; to be cleaned later
	for (size_t i = 0; i < v.n_regions; i++)
	{
		short t = 16 * region_types[i];

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
		fprintf(stderr, "Rasterization done\n");
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
		char top    = LAND_SAME(i-1,j);
		char right  = LAND_SAME(i,j+1);
		char bottom = LAND_SAME(i+1,j);
		char left   = LAND_SAME(i,j-1);
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

	if (w->settings->verbosity >= 1)
		fprintf(stderr, "Map is ready\n");
}

void world_start(world_t* w)
{
	universe_t* u = w->universe;

	// BEGIN character generation
	pool_t* p = &w->objects;
	size_t n_characters = 1 + w->settings->bots_count;
	for (size_t i = 0; i < n_characters; i++)
	{
		character_t* c = character_new(p, -1);

		int type = rand() % u->n_characters;
		character_init(c, &u->characters[type], u, w);
		character_setPosition(c, cfrnd(w->o.w-20), cfrnd(w->o.h-20));
	}
	if (w->settings->verbosity >= 3)
		fprintf(stderr, "Generated %u characters\n", (unsigned) n_characters);
	// END character generation
}

void world_randMine(world_t* w, int type)
{
	mine_t* m = mine_new(&w->objects, -1);
	mine_init(m, &w->universe->mines[type]);
	while (1)
	{
		float x = cfrnd(w->o.w - 32);
		float y = cfrnd(w->o.h - 32) + 16;
		int t = world_getLandXY(w, x, y) / 16;
		if (t == 4 || t == 10)
			continue;

		chunk_t* c = world_chunkXY(w, x, y);
		m->o.x = x;
		m->o.y = y;
		if (chunk_pushMine(c, m))
			break;
	}
}
