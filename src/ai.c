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

#include "ai.h"

#include <stdlib.h>
#include <stdio.h>

#include "string.h"

void ai_init(ai_t* ai)
{
	ai->name = NULL;
	transform_init(&ai->inventory);
	ai->building = -1;
}

void ai_exit(ai_t* ai)
{
	transform_exit(&ai->inventory);
	free(ai->name);
}

void ai_load(ai_t* ai, const char* filename)
{
	FILE* f = fopen(filename, "r");
	if (f == NULL)
	{
		fprintf(stderr, "Could not open AI file '%s'\n", filename);
		exit(1);
	}

	char* s = strrchr(filename, '/');
	char* e = strrchr(filename, '.');
	if (s != NULL && e != NULL)
	{
		*e = 0;
		ai->name = strdup(s+1);
	}

	int age = 0;

	char* line = NULL;
	size_t n_line = 0;
	while (getline(&line, &n_line, f) > 0)
	{
		float val = 0;
		char* strval = strchr(line, '=');
		if (strval != NULL)
		{
			*strval = 0;
			val = strtol(strval+1, NULL, 0);
		}

		int id = 0;
		char* strid = strchr(line, '_');
		if (strid != NULL)
		{
			*strid = 0;
			id = strtol(strid+1, NULL, 0);
		}

		char* key = line;

		if (strcmp(key, "Age") == 0)
		{
			age = id;
		}

		if (age != 1)
			continue;

		if (strcmp(key, "Ressource") == 0)
		{
			transform_req(&ai->inventory, id - 1, val, 0);
		}
		else if (strcmp(key, "Objet") == 0)
		{
			transform_req(&ai->inventory, id - 1, val, 1);
		}
		else if (strcmp(key, "Batiment") == 0)
		{
			ai->building = id - 1;
		}
	}

	free(line);
	fclose(f);
}

char ai_get(character_t* c, component_t* p, float amount)
{
	universe_t* u = c->w->universe;

	amount *= p->amount;
	amount -= (p->is_item ? c->inventory.items : c->inventory.materials)[p->id];
	if (amount < 0)
		return 0;

	// gather if possible
	kindOf_mine_t* m = universe_mineFor(u, p->id, p->is_item);
	if (m != NULL)
	{
		character_goMine(c, m);
		return 1;
	}

	// if the current building cannot obtain the component, build one which can
	building_t* b = building_get(&c->w->objects, c->hasBuilding);
	transform_t* tr = b == NULL ? NULL : kindOf_building_available(b->t, p->id, p->is_item);
	if (tr == NULL)
	{
		kindOf_building_t* b = universe_buildFor(u, p->id, p->is_item);
		if (b == NULL)
		{
			const char* name = p->is_item ? u->items[p->id].name : u->materials[p->id].name;
			fprintf(stderr, "%s: I do not know how to make %s\n", c->ai->name, name);
			return 1;
		}

		// materials to be gather before building
		transform_t total;
		transform_init(&total);

		// first, gather the materials for the component
		tr = kindOf_building_available(b, p->id, p->is_item);
		transform_add(&total, tr, amount);

		// then, gather the materials for the building
		transform_add(&total, &b->build, 1);

		// do gather
		char isreq = ai_getreq(c, &total, 1);
		transform_exit(&total);
		if (isreq)
			return 1;

		// build
		character_buildAuto(c, b);
		return 1;
	}

	// go in the building
	if (c->inBuilding != c->hasBuilding)
	{
		c->go_o = b->o.uuid;
		return 1;
	}

	// enqueue item
	if (p->is_item && b->work_n == 0)
	{
		int nth = tr - b->t->items;
		building_work_enqueue(b, nth);
	}

	// work
	return 1;
}

char ai_getreq(character_t* c, transform_t* tr, float amount)
{
	for (int i = 0; i < tr->n_req; i++)
		if (ai_get(c, &tr->req[i], amount))
			return 1;
	return 0;
}

char ai_build(character_t* c, int id)
{
	universe_t* u = c->w->universe;
	kindOf_building_t* t = &u->buildings[id];

	building_t* b = building_get(&c->w->objects, c->hasBuilding);
	if (b != NULL && b->t == t)
		return 0;

	if (ai_getreq(c, &t->build, 1))
		return 1;

	character_buildAuto(c, t);
	return 1;
}

char ai_do(ai_t* ai, character_t* c)
{
	transform_t* tr = &ai->inventory;

	// eat apples
	float max = character_maxOfStatus(c, ST_STAMINA);
	float threshold = max * 0.5;
	if (c->statuses[ST_STAMINA] < threshold)
	{
		while (c->statuses[ST_STAMINA] < max && character_eat(c, 1));
		return 1;
	}
	// ensures the AI always has apples to eat
	static component_t apples = {.id=1, .amount=1};
	if (ai_get(c, &apples, 5))
		return 1;

	if (ai_getreq(c, tr, 1))
		return 1;

	if (ai->building >= 0 && ai_build(c, ai->building))
		return 1;

	if (c->inBuilding != c->hasBuilding)
	{
		building_t* b = building_get(&c->w->objects, c->hasBuilding);
		if (b != NULL)
			c->go_o = b->o.uuid;
		return 1;
	}

	return 0;
}
