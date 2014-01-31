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

#include "world_save.h"

#include <stdlib.h>

#include "world_gen.h"

static void save_object(object_t* o, FILE* f)
{
	fprintf(f, "%li %f %f %f %f", o->uuid, o->x, o->y, o->w, o->h);
}
void world_save(world_t* w, FILE* f)
{
	universe_t* u = w->universe;

	// seed
	fprintf(f, "seed = %#x\n", w->settings->seed);

	// characters
	pool_t* p = &w->objects;
	size_t n_characters = 0;
	for (size_t i = 0; i < p->n_objects; i++)
		if (character_get(p, i) != NULL)
			n_characters++;
	fprintf(f, "%u characters\n", (unsigned) n_characters);
	for (size_t i = 0; i < p->n_objects; i++)
	{
		character_t* c = character_get(p, i);
		if (c == NULL)
			continue;

		int t = c->t - u->characters;

		save_object(&c->o, f);
		fprintf(f, " %i %hhi %f %f %li %li %li\n",
			t,
			c->alive, c->go_x, c->go_y, c->go_o,
			c->hasBuilding, c->inBuilding);
	}

	// buildings
	size_t n_buildings = 0;
	for (size_t i = 0; i < p->n_objects; i++)
		if (building_get(p, i) != NULL)
			n_buildings++;
	fprintf(f, "%u buildings\n", (unsigned) n_buildings);
	for (size_t i = 0; i < p->n_objects; i++)
	{
		building_t* b = building_get(p, i);
		if (b == NULL)
			continue;

		save_object(&b->o, f);
		fprintf(f, " %li %f %f\n",
			b->owner, b->build_progress, b->life);
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

	// characters
	pool_t* p = &w->objects;
	unsigned n_characters;
	CLINE("%u characters\n", &n_characters);
	for (size_t i = 0; i < n_characters; i++)
	{
		object_t o;
		int t;
		signed char alive;
		float go_x;
		float go_y;
		uuid_t go_o;
		uuid_t hasBuilding;
		uuid_t inBuilding;
		o.t = O_CHARACTER;
		CLINE("%li %f %f %f %f %i %hhi %f %f %li %li %li\n",
			&o.uuid, &o.x, &o.y, &o.w, &o.h,
			&t,
			&alive, &go_x, &go_y, &go_o,
			&hasBuilding, &inBuilding);

		// uuid = i
		character_t* c = character_new(p, o.uuid);
		character_init(c, &u->characters[t], u, w);
		c->o = o;
		c->alive = alive;
		c->go_x = go_x;
		c->go_y = go_y;
		c->go_o = go_o;
		c->hasBuilding = hasBuilding;
		c->inBuilding = inBuilding;
	}

	unsigned n_buildings;
	CLINE("%u buildings\n", &n_buildings);
	for (size_t i = 0; i < n_buildings; i++)
	{
		object_t o;
		uuid_t owner;
		float build_progress;
		float life;
		o.t = O_BUILDING;
		CLINE("%li %f %f %f %f %li %f %f\n",
			&o.uuid, &o.x, &o.y, &o.w, &o.h,
			&owner, &build_progress, &life);

		building_t* b = building_new(p, o.uuid);
		building_init(b, NULL, owner, 0, 0);
		b->o = o;
		b->build_progress = build_progress;
		b->life = life;
	}
}
