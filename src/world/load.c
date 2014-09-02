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

#include "load.h"

#include <stdlib.h>

void load_listb(cfg_t* cfg, char* l, size_t n)
{
	if (cfg == NULL)
		return;

	if (cfg->n_entries < n)
		n = cfg->n_entries;

	for (size_t i = 0; i < cfg->n_entries; i++)
		l[i] = cfg->entries[i].d.str[0] == 'f' ? 0 : 1;
}
void load_listi(cfg_t* cfg, int* l, size_t n)
{
	if (cfg == NULL)
		return;

	if (cfg->n_entries < n)
		n = cfg->n_entries;

	for (size_t i = 0; i < cfg->n_entries; i++)
		l[i] = atoi(cfg->entries[i].d.str);
}
void load_listf_dic(cfg_t* cfg, float* l, size_t n)
{
	if (cfg == NULL)
		return;

	for (size_t i = 0; i < cfg->n_entries; i++)
	{
		cfg_entry_t* e = &cfg->entries[i];
		unsigned int idx = atoi(e->key);
		float v = atof(e->d.str);

		if (idx >= n)
		{
			fprintf(stderr, "Invalid index %u (max %u) with value (%.9g)\n", idx, (unsigned) n, v);
		}

		l[idx] = v;
	}
}
void load_object(cfg_t* cfg, object_t* o)
{
	o->uuid = cfg_get_int  (cfg, "uuid");
	o->x    = cfg_get_float(cfg, "x");
	o->y    = cfg_get_float(cfg, "y");
	o->w    = cfg_get_float(cfg, "w");
	o->h    = cfg_get_float(cfg, "h");
}
void load_inventory(cfg_t* cfg, inventory_t* inv, universe_t* u)
{
	inv->money = cfg_get_float(cfg, "money");

	cfg_t* materials = cfg_get_group(cfg, "materials");
	load_listf_dic(materials, inv->materials, u->n_materials);

	cfg_t* items = cfg_get_group(cfg, "items");
	load_listf_dic(items, inv->items, u->n_items);
}
void load_ai_data(cfg_t* cfg, ai_data_t* d)
{
	d->step    = cfg_get_int(cfg, "step");
	d->collect = cfg_get_int(cfg, "collect");
	d->sell    = cfg_get_int(cfg, "sell");
}
void load_character(cfg_t* cfg, character_t* c)
{
	universe_t* u = c->w->universe;

	load_object(cfg, &c->o);

	int ai = cfg_get_int(cfg, "ai");
	c->ai = ai < 0 ? NULL : &u->bots[ai];

	cfg_t* ai_data = cfg_get_group(cfg, "ai_data");
	load_ai_data(ai_data, &c->ai_data);

	c->alive       = cfg_get_bool(cfg, "alive");
	c->go_x        = cfg_get_int (cfg, "target_x");
	c->go_y        = cfg_get_int (cfg, "target_y");
	c->go_o        = cfg_get_int (cfg, "target");
	c->hasBuilding = cfg_get_int (cfg, "home");
	c->inBuilding  = cfg_get_int (cfg, "inBuilding");

	load_inventory(cfg, &c->inventory, u);

	cfg_t* skills = cfg_get_group(cfg, "skills");
	load_listf_dic(skills, c->skills, u->n_skills);

	cfg_t* statuses= cfg_get_group(cfg, "statuses");
	load_listf_dic(statuses, c->statuses, N_STATUSES);

	cfg_t* equipment = cfg_get_group(cfg, "equipment");
	load_listi(equipment, c->equipment, u->n_slots);
}
void load_building(cfg_t* cfg, building_t* b)
{
	universe_t* u = b->w->universe;

	load_object(cfg, &b->o);

	b->owner          = cfg_get_int  (cfg, "owner");
	b->build_progress = cfg_get_float(cfg, "build_progress");
	b->life           = cfg_get_float(cfg, "life");
	b->work_progress  = cfg_get_float(cfg, "work_progress");

	load_inventory(cfg, &b->inventory, u);
	building_update(b);

	cfg_t* work_list = cfg_get_group(cfg, "work_list");
	if (work_list != NULL)
	{
		for (size_t i = 0; i < work_list->n_entries; i++)
		{
			cfg_entry_t* e = &work_list->entries[i];
			building_work_enqueue(b, atoi(e->d.str));
		}
	}
}
void load_world(cfg_t* cfg, world_t* w)
{
	universe_t* u = w->universe;

	w->seed = cfg_get_int(cfg, "seed");
	w->rows = cfg_get_int(cfg, "rows");
	w->cols = cfg_get_int(cfg, "cols");
	world_genmap(w, w->seed);

	cfg_t* characters = cfg_get_group(cfg, "characters");
	if (characters != NULL)
	for (size_t i = 0; i < characters->n_entries; i++)
	{
		cfg_t* cfg = characters->entries[i].d.group;

		uuid_t uuid = cfg_get_int(cfg, "uuid");
		character_t* c = character_new(&w->objects, uuid);

		int t = cfg_get_int(cfg, "type");
		character_init(c, w, &u->characters[t]);
		load_character(cfg, c);
	}

	cfg_t* buildings = cfg_get_group(cfg, "buildings");
	if (buildings != NULL)
	for (size_t i = 0; i < buildings->n_entries; i++)
	{
		cfg_t* cfg = buildings->entries[i].d.group;

		uuid_t uuid = cfg_get_int(cfg, "uuid");
		building_t* b = building_new(&w->objects, uuid);

		int t = cfg_get_int(cfg, "type");
		building_init(b, w, &u->buildings[t], 0, 0, 0); // TODO
		load_building(cfg, b);

		chunk_pushBuilding(world_chunkXY(w, b->o.x-b->o.w/2, b->o.y-b->o.h), b);
		chunk_pushBuilding(world_chunkXY(w, b->o.x-b->o.w/2, b->o.y       ), b);
		chunk_pushBuilding(world_chunkXY(w, b->o.x+b->o.w/2, b->o.y-b->o.h), b);
		chunk_pushBuilding(world_chunkXY(w, b->o.x+b->o.w/2, b->o.y       ), b);
	}
}
