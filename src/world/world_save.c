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
#include "../mem.h"

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
		int ai = c->ai == NULL ? -1 : c->ai - u->bots;

		save_object(&c->o, f);
		fprintf(f, " %i %i %hhi %f %f %li %li %li\n",
			t, ai,
			c->alive, c->go_x, c->go_y, c->go_o,
			c->hasBuilding, c->inBuilding);

		// inventory
		for (size_t i = 0; i < u->n_materials; i++)
			fprintf(f, "%f,", c->inventory.materials[i]);
		fprintf(f, "\n");
		for (size_t i = 0; i < u->n_items; i++)
			fprintf(f, "%f,", c->inventory.items[i]);
		fprintf(f, "\n");

		// skills
		for (size_t i = 0; i < u->n_skills; i++)
			fprintf(f, "%f,", c->skills[i]);
		fprintf(f, "\n");

		// equipment
		for (size_t i = 0; i < u->n_slots; i++)
			fprintf(f, "%i,", c->equipment[i]);
		fprintf(f, "\n");
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

		int t = b->t - u->buildings;

		save_object(&b->o, f);
		fprintf(f, " %i %li %f %f %f %u\n",
			t,
			b->owner, b->build_progress, b->life,
			b->work_progress, (unsigned) b->work_n);
		for (size_t i = 0; i < b->work_n; i++)
			fprintf(f, "%i,", b->work_list[i]);
		fprintf(f, "\n");
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
		int ai;
		signed char alive;
		float go_x;
		float go_y;
		uuid_t go_o;
		uuid_t hasBuilding;
		uuid_t inBuilding;
		o.t = O_CHARACTER;
		CLINE("%li %f %f %f %f %i %i %hhi %f %f %li %li %li\n",
			&o.uuid, &o.x, &o.y, &o.w, &o.h,
			&t, &ai,
			&alive, &go_x, &go_y, &go_o,
			&hasBuilding, &inBuilding);

		// uuid = i
		character_t* c = character_new(p, o.uuid);
		character_init(c, &u->characters[t], u, w);
		c->o = o;
		c->ai = ai < 0 ? NULL : &u->bots[ai];
		c->alive = alive;
		c->go_x = go_x;
		c->go_y = go_y;
		c->go_o = go_o;
		c->hasBuilding = hasBuilding;
		c->inBuilding = inBuilding;

		// inventory
		for (size_t i = 0; i < u->n_materials; i++)
			CLINE("%f,", &c->inventory.materials[i]);
		CLINE("\n");
		for (size_t i = 0; i < u->n_items; i++)
			CLINE("%f,", &c->inventory.items[i]);
		CLINE("\n");

		// skills
		for (size_t i = 0; i < u->n_skills; i++)
			CLINE("%f,", &c->skills[i]);
		CLINE("\n");

		// equipment
		for (size_t i = 0; i < u->n_slots; i++)
			CLINE("%i,", &c->equipment[i]);
		CLINE("\n");
	}

	unsigned n_buildings;
	CLINE("%u buildings\n", &n_buildings);
	for (size_t i = 0; i < n_buildings; i++)
	{
		object_t o;
		int t;
		uuid_t owner;
		float build_progress;
		float life;
		float work_progress;
		unsigned work_n;
		o.t = O_BUILDING;
		CLINE("%li %f %f %f %f %i %li %f %f %f %u\n",
			&o.uuid, &o.x, &o.y, &o.w, &o.h,
			&t,
			&owner, &build_progress, &life,
			&work_progress, &work_n);

		building_t* b = building_new(p, o.uuid);
		building_init(b, &u->buildings[t], owner, 0, 0);
		b->o = o;
		b->build_progress = build_progress;
		b->life = life;
		b->work_progress = work_progress;
		b->work_n = work_n;

		b->work_list = CALLOC(int, work_n);
		for (size_t i = 0; i < work_n; i++)
			CLINE("%i,", &b->work_list[i]);
		CLINE("\n");
	}
}
