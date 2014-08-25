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
			transform_req(&ai->inventory, MATERIAL, id - 1, val);
		}
		else if (strcmp(key, "Objet") == 0)
		{
			transform_req(&ai->inventory, ITEM, id - 1, val);
		}
		else if (strcmp(key, "Batiment") == 0)
		{
			ai->building = id - 1;
		}
	}

	free(line);
	fclose(f);
}

char ai_get(character_t* c, char is_item, int id, float amount, char keep)
{
	universe_t* u = c->w->universe;

	amount -= inventory_get(&c->inventory, is_item, id);
	if (amount <= 0)
		return 0;

	// gather if possible
	kindOf_mine_t* m = universe_mineFor(u, is_item, id);
	if (m != NULL)
	{
		character_goMine(c, m);
		return 1;
	}

	// if the current building cannot obtain the component, build one which can
	building_t* b = building_get(&c->w->objects, c->hasBuilding);
	transform_t* tr = b == NULL ? NULL : kindOf_building_canMake(b->t, is_item, id);
	if (tr == NULL)
	{
		// do not replace the building
		if (keep)
		{
			float price = is_item ? u->items[id].price : u->materials[id].price;
			price *= amount;
			if (c->inventory.money < price)
			{
				if (c->inBuilding != c->hasBuilding)
					c->go_o = c->hasBuilding;
				return 1;
			}

			// try and buy it
			building_t* b = world_findSale(c->w, c, is_item, id);
			if (b == NULL)
				return 1;

			if (c->inBuilding != b->o.uuid)
			{
				c->go_o = b->o.uuid;
				return 1;
			}

			building_take(b, is_item, id, amount, &c->inventory, 0);
			return 1;
		}

		kindOf_building_t* b = universe_buildFor(u, is_item, id);
		if (b == NULL)
		{
			const char* name = is_item ? u->items[id].name : u->materials[id].name;
			fprintf(stderr, "%s: I do not know how to make %s\n", c->ai->name, name);
			return 1;
		}

		// materials to be gather before building
		transform_t total;
		transform_init(&total);

		// first, gather the non-base materials for the component
		tr = kindOf_building_canMake(b, is_item, id);
		for (int i = 0; i < tr->n_req; i++)
		{
			component_t* p = &tr->req[i];
			if (universe_mineFor(u, p->is_item, p->id) == NULL)
				transform_req(&total, p->is_item, p->id, p->amount*amount);
		}

		// then, gather the materials for the building
		transform_add(&total, &b->build, 1);

		// do gather
		char isreq = ai_getreq(c, &total, 1, keep);
		transform_exit(&total);
		if (isreq)
			return 1;

		// build
		character_buildAuto(c, b);
		return 1;
	}

	if (ai_getreq(c, tr, amount, keep))
		return 1;

	// go in the building
	if (c->inBuilding != c->hasBuilding)
	{
		c->go_o = b->o.uuid;
		return 1;
	}

	// enqueue item
	if (is_item && b->work_n == 0)
	{
		int nth = tr - b->t->items;
		building_work_enqueue(b, nth);
	}

	// work
	return 1;
}

char ai_getreq(character_t* c, transform_t* tr, float amount, char keep)
{
	for (int i = 0; i < tr->n_req; i++)
	{
		component_t* p = &tr->req[i];
		if (ai_get(c, p->is_item, p->id, p->amount*amount, keep))
			return 1;
	}
	return 0;
}

char ai_build(character_t* c, int id)
{
	universe_t* u = c->w->universe;
	kindOf_building_t* t = &u->buildings[id];

	building_t* b = building_get(&c->w->objects, c->hasBuilding);
	if (b != NULL && b->t == t)
		return 0;

	if (ai_getreq(c, &t->build, 1, 0))
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
	if (ai_get(c, MATERIAL, 1, 5.0, 1))
		return 1;

	// get preliminary components
	while (c->ai_data.step < tr->n_req)
	{
		component_t* p = &tr->req[c->ai_data.step];
		if (ai_get(c, p->is_item, p->id, p->amount, 0))
			return 1;
		else
			c->ai_data.step++;
	}

	// make building for job
	if (ai->building >= 0 && ai_build(c, ai->building))
		return 1;
	building_t* b = building_get(&c->w->objects, c->hasBuilding);
	if (b == NULL)
	{
		fprintf(stderr, "Where did my house go? :(\n");
		return 0;
	}
	if (c->inBuilding == c->hasBuilding)
	{
		building_withdraw(b, &c->inventory);
	}
	else if (b->build_progress < 1)
	{
		c->go_o = b->o.uuid;
		return 1;
	}

	// make items as a job
	int n = b->t->n_items;
	if (n > 0)
	{
		if (b->work_n == 0)
		{
			if (c->ai_data.sell > 0)
				building_put(b, ITEM, c->ai_data.sell-1, 1.0, &c->inventory, 1);

			int nth = rand() % n;
			building_work_enqueue(b, nth);
			c->ai_data.collect = 1;
			c->ai_data.sell = b->t->items[nth].res[0].id + 1;
		}
		if (c->ai_data.collect)
		{
			transform_t* tr = &b->t->items[b->work_list[0]];
			if (ai_getreq(c, tr, 1, 1))
				return 1;
			else
				c->ai_data.collect = 0;
		}
	}

	// transform materials as a job
	tr = &b->t->make;
	if (tr != NULL)
	{
		if (!c->ai_data.collect)
			c->ai_data.collect = transform_ratio(tr, &c->inventory, -1) <= 0;

		if (c->ai_data.collect)
		{
			for (int i = 0; i < tr->n_req; i++)
			{
				component_t* p = &tr->req[i];
				if (ai_get(c, p->is_item, p->id, p->amount*5, 1))
					return 1;
			}
			c->ai_data.collect = 0;
		}
	}

	// go home
	if (c->inBuilding != c->hasBuilding)
	{
		c->go_o = b->o.uuid;
		return 1;
	}

	return 1;
}
