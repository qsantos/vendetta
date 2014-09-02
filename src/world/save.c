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

#include "save.h"

void save_listb(cfg_t* cfg, char* l, size_t n)
{
	for (size_t i = 0; i < n; i++)
		cfg_put_bool(cfg, NULL, l[i]);
}
void save_listi(cfg_t* cfg, int* l, size_t n)
{
	for (size_t i = 0; i < n; i++)
		cfg_put_int(cfg, NULL, l[i]);
}
void save_listf_dic(cfg_t* cfg, float* l, size_t n, float default_value)
{
	for (size_t i = 0; i < n; i++)
	{
		if (l[i] != default_value)
		{
			char buffer[32];
			snprintf(buffer, 32, "%zu", i);
			cfg_put_float(cfg, buffer, l[i]);
		}
	}
}
void save_object(cfg_t* cfg, object_t* o)
{
	cfg_put_int  (cfg, "uuid", o->uuid);
	cfg_put_float(cfg, "x",    o->x);
	cfg_put_float(cfg, "y",    o->y);
	cfg_put_float(cfg, "w",    o->w);
	cfg_put_float(cfg, "h",    o->h);
}
void save_inventory(cfg_t* cfg, inventory_t* inv, universe_t* u)
{
	cfg_put_float(cfg, "money", inv->money);

	cfg_t* materials = cfg_new();
	save_listf_dic(materials, inv->materials, u->n_materials, 0.f);
	cfg_put_group(cfg, "materials", materials);

	cfg_t* items = cfg_new();
	save_listf_dic(items, inv->items, u->n_items, 0.f);
	cfg_put_group(cfg, "items", items);
}
void save_ai_data(cfg_t* cfg, ai_data_t* d)
{
	cfg_put_int(cfg, "step",    d->step);
	cfg_put_int(cfg, "collect", d->collect);
	cfg_put_int(cfg, "sell",    d->sell);
}
void save_character(cfg_t* cfg, character_t* c)
{
	universe_t* u = c->w->universe;

	save_object(cfg, &c->o);

	int t = c->t - u->characters;
	cfg_put_int(cfg, "type", t);

	int ai = c->ai == NULL ? -1 : c->ai - u->bots;
	cfg_put_int(cfg, "ai", ai);

	cfg_t* ai_data = cfg_new();
	save_ai_data(ai_data, &c->ai_data);
	cfg_put_group(cfg, "ai_data", ai_data);

	cfg_put_bool(cfg, "alive",      c->alive);
	cfg_put_int (cfg, "target_x",   c->go_x);
	cfg_put_int (cfg, "target_y",   c->go_y);
	cfg_put_int (cfg, "target",     c->go_o);
	cfg_put_int (cfg, "home",       c->hasBuilding);
	cfg_put_int (cfg, "inBuilding", c->inBuilding);

	save_inventory(cfg, &c->inventory, u);

	cfg_t* skills = cfg_new();
	save_listf_dic(skills, c->skills, u->n_skills, 20.f);
	cfg_put_group(cfg, "skills", skills);

	cfg_t* statuses = cfg_new();
	save_listf_dic(statuses, c->statuses, N_STATUSES, 20.f);
	cfg_put_group(cfg, "statuses", statuses);

	cfg_t* equipment = cfg_new();
	save_listi(equipment, c->equipment, u->n_slots);
	cfg_put_group(cfg, "equipment", equipment);
}
void save_building(cfg_t* cfg, building_t* b)
{
	universe_t* u = b->w->universe;

	save_object(cfg, &b->o);

	int t = b->t - u->buildings;
	cfg_put_int(cfg, "type", t);

	cfg_put_int  (cfg, "owner",          b->owner);
	cfg_put_float(cfg, "build_progress", b->build_progress);
	cfg_put_float(cfg, "life",           b->life);
	cfg_put_float(cfg, "work_progress",  b->work_progress);

	save_inventory(cfg, &b->inventory, u);

	if (b->work_n > 0)
	{
		cfg_t* work_list = cfg_new();
		for (size_t i = 0; i < b->work_n; i++)
			cfg_put_int(work_list, NULL, b->work_list[i]);
		cfg_put_group(cfg, "work_list", work_list);
	}
}
void save_world(cfg_t* cfg, world_t* w)
{
	cfg_put_int(cfg, "seed", w->seed);
	cfg_put_int(cfg, "rows", w->rows);
	cfg_put_int(cfg, "cols", w->cols);

	cfg_t* characters = cfg_new();
	cfg_t* buildings  = cfg_new();

	pool_t* p = &w->objects;
	for (size_t i = 0; i < p->n_objects; i++)
	{
		object_t* o = p->objects[i];
		cfg_t* cfg = cfg_new();
		switch (o->t)
		{
			case O_NONE:
				break;
			case O_PROJECTILE:
			case O_BUILDING:
				save_building(cfg, (building_t*) o);
				cfg_put_group(buildings, NULL, cfg);
				break;
			case O_MINE:
				break;
			case O_CHARACTER:
				save_character(cfg, (character_t*) o);
				cfg_put_group(characters, NULL, cfg);
				break;
			case O_CHUNK:
			case O_WORLD:
				break;
		}
	}

	cfg_put_group(cfg, "characters", characters);
	cfg_put_group(cfg, "buildings",  buildings);
}
