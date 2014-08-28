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

#include "character.h"

#include <string.h>

#include "../math.h"
#include "../mem.h"
#include "../rand.h"

void character_init(character_t* c, world_t* w, kindOf_character_t* t)
{
	c->o.t = O_CHARACTER;
	c->o.w = 24;
	c->o.h = 32;

	c->w = w;
	c->t = t;

	c->alive = 1;

	c->go_o = -1;
	c->dir = D_SOUTH;
	c->step = 5; // standing still
	c->attackDelay = 0;
	c->inWater = 0;

	c->attack = 0;

	c->ai = NULL;
	memset(&c->ai_data, 0, sizeof(ai_data_t));

	universe_t* u = w->universe;

	inventory_init(&c->inventory, u->n_materials, u->n_items);
	c->inventory.money = 100;
	c->hasBuilding = -1;
	c->inBuilding  = -1;

	c->skills = CALLOC(skill_t, u->n_skills);
	for (size_t i = 0; i < u->n_skills; i++)
		c->skills[i] = 20;

	c->equipment = CALLOC(int, u->n_slots);
	for (size_t i = 0; i < u->n_slots; i++)
		c->equipment[i] = -1;

	for (int i = 0; i < N_STATUSES; i++)
	{
		float max = character_maxOfStatus(c, i);
		c->statuses[i] = max;
	}
}

void character_exit(character_t* c)
{
	if (c == NULL)
		return;

	free(c->equipment);
	free(c->skills);
	inventory_exit(&c->inventory);
}

static float vitality_malus(character_t* c, int i)
{
	float max = character_maxOfStatus(c, i);
	float s = c->statuses[i] / max;
	if (s < 0.10) return 0.450;
	if (s < 0.25) return 0.250;
	if (s < 0.50) return 0.125;
	return 0;
}
float character_vitality(character_t* c)
{
	float ret = 1;
	ret -= vitality_malus(c, ST_HEALTH);
	ret -= vitality_malus(c, ST_STAMINA);
	ret -= vitality_malus(c, ST_MORAL);
	return fmax(ret, 0.1);
}

float character_getSkill(character_t* c, int skill)
{
	float ret = c->skills[skill];

	// bonuses
	universe_t* u = c->w->universe;
	for (size_t i = 0; i < u->n_slots; i++)
	{
		int item = c->equipment[i];
		if (item < 0)
			continue;
		kindOf_item_t* t = &u->items[item];
		effect_t* e = &t->effect;
		for (size_t j = 0; j < e->n_skills; j++)
			if (e->skills[j] == skill)
			{
				ret += e->bonuses[j];
				break;
			}
	}

	return ret / 20;
}

float character_maxOfStatus(character_t* c, int s)
{
	float ret = 20;

	// bonuses
	universe_t* u = c->w->universe;
	for (size_t i = 0; i < u->n_slots; i++)
	{
		int item = c->equipment[i];
		if (item < 0)
			continue;
		kindOf_item_t* t = &u->items[item];
		effect_t* e = &t->effect;
		ret += e->status_bonus[s];
	}

	return ret;
}

float character_maxOfMaterial(character_t* c, kindOf_material_t* m)
{
	float ret = 20 * character_getSkill(c, m->skill);

	// bonuses
	universe_t* u = c->w->universe;
	for (size_t i = 0; i < u->n_slots; i++)
	{
		int item = c->equipment[i];
		if (item < 0)
			continue;
		kindOf_item_t* t = &u->items[item];
		effect_t* e = &t->effect;
		ret += e->max_material;
	}

	return ret;
}

float character_armor(character_t* c)
{
	float ret = 0;

	// bonuses
	universe_t* u = c->w->universe;
	for (size_t i = 0; i < u->n_slots; i++)
	{
		int item = c->equipment[i];
		if (item < 0)
			continue;
		kindOf_item_t* t = &u->items[item];
		effect_t* e = &t->effect;
		ret += e->armor;
	}

	return ret;
}

void character_addStatus(character_t* c, int s, float q)
{
	float max = character_maxOfStatus(c, s);
	float n = c->statuses[s] + q;
	c->statuses[s] = fmin(fmax(n, 0), max);
}

void character_weary(character_t* c, float f)
{
	character_addStatus(c, ST_STAMINA, -f);
	character_addStatus(c, ST_STAMINA, -f/3);
}

void character_train(character_t* c, int skill, float work)
{
	c->skills[skill] += work/50;
	character_weary(c, 0.1 * work);
}

void character_stop(character_t* c)
{
	c->go_x = c->o.x;
	c->go_y = c->o.y;
	c->go_o = -1;
}

void character_move(character_t* c, float dx, float dy)
{
	character_goAt(c, c->o.x+dx, c->o.y+dy);
}

void character_goAt(character_t* c, float x, float y)
{
	c->go_x = x;
	c->go_y = y;
	c->go_o = -1;
}

void character_goto(character_t* c, uuid_t uuid)
{
	c->attack = 0;
	c->go_o = uuid;
}

void character_attack(character_t* c, uuid_t uuid)
{
	c->attack = 1;
	c->go_o = uuid;
}

char character_eat(character_t* c, int material)
{
	universe_t* u = c->w->universe;

	kindOf_material_t* m = &u->materials[material];
	if (!m->edible)
		return 0;

	if (c->inventory.materials[material] < 1)
		return 0;

	c->inventory.materials[material] -= 1;
	for (int i = 0; i < N_STATUSES; i++)
		character_addStatus(c, i, m->eatBonus[i]);
	return 1;
}

void character_eatFor(character_t* c, int status)
{
	universe_t* u = c->w->universe;
	for (size_t i = 0; i < u->n_materials; i++)
	{
		kindOf_material_t* m = &u->materials[i];
		if (m->eatBonus[status] > 0 && character_eat(c, i))
			return;
	}
}

void character_setPosition(character_t* c, float x, float y)
{
	c->o.x = x;
	c->o.y = y;
	c->go_x = x;
	c->go_y = y;

	// TODO
	// quickfix: check if already in water
	{
	world_t* w = c->w;
	short l = world_getLandXY(w, c->o.x, c->o.y)/16;
	if (l == 10)
	{
		float y = c->o.y - 6;
		if (world_getLandXY(w, c->o.x-c->o.w/2, y)/16 == l &&
		    world_getLandXY(w, c->o.x+c->o.w/2, y)/16 == l)
		{
			c->inWater = 1;
		}
	}
	}
}

float character_attacked(character_t* c, float work)
{
	float defense = c->statuses[ST_DEFENSE];
	defense = fmax(defense, character_armor(c));

	character_addStatus(c, ST_DEFENSE, -work);
	character_addStatus(c, ST_HEALTH, work - defense);
	if (c->statuses[ST_HEALTH] <= 0)
		c->alive = 0;
	return work;
}

void character_goMine(character_t* c, kindOf_mine_t* t)
{
	mine_t* m = mine_get(&c->w->objects, c->go_o);
	if (m != NULL && m->t == t)
		return;

	m = world_findMine(c->w, c->o.x, c->o.y, t);
	if (m != NULL)
		c->go_o = m->o.uuid;
}

char character_buildAuto(character_t* c, kindOf_building_t* t)
{
	if (!transform_check(&t->build, &c->inventory))
		return 0;

	character_delHome(c);

	float w = c->w->o.w;
	float h = c->w->o.h;
	float max = sqrt(w*w + h*h) * 1.5;
	for (float radius = 50; radius < max; radius *= 1.5)
	{
		float x = c->o.x + cfrnd(radius);
		float y = c->o.y + cfrnd(radius);

		if (character_buildAt(c, t, x, y))
			return 1;
	}

	return 0;
}

char character_buildAt(character_t* c, kindOf_building_t* t, float x, float y)
{
	if (!transform_check(&t->build, &c->inventory))
		return 0;

	character_delHome(c);

	building_t* b = world_addBuilding(c->w, x, y, t, c);
	if (b == NULL)
		return 0;

	transform_apply(&t->build, &c->inventory, 1);
	c->hasBuilding = b->o.uuid;
	return 1;
}

char character_delHome(character_t* c)
{
	building_t* b = building_get(&c->w->objects, c->hasBuilding);
	if (b == NULL)
	{
		c->hasBuilding = -1;
		return 0;
	}

	world_delBuilding(c->w, b);
	c->hasBuilding = -1;
	return 1;
}

building_t* character_get_inBuilding(character_t* c)
{
	if (c->inBuilding < 0)
		return NULL;

	building_t* b = building_get(&c->w->objects, c->inBuilding);
	if (b == NULL)
		c->inBuilding = -1;
	return b;
}

building_t* character_get_hasBuilding(character_t* c)
{
	if (c->hasBuilding < 0)
		return NULL;

	building_t* b = building_get(&c->w->objects, c->hasBuilding);
	if (b == NULL)
		c->hasBuilding = -1;
	return b;
}

size_t character_currentAction(character_t* c, char* buffer, size_t n)
{
	size_t cur = 0;

	object_t* o = pool_get(&c->w->objects, c->go_o);
	universe_t* u = c->w->universe;
	char moving = c->go_x != c->o.x || c->go_y != c->o.y;
	if (o == NULL)
	{
		if (moving)
			cur += snprintf(buffer+cur, n-cur, "Marcher");
		else
			cur += snprintf(buffer+cur, n-cur, "Attendre");
	}
	else if (o->t == O_MINE)
	{
		mine_t* m = (mine_t*) o;

		if (moving)
		{
			cur += snprintf(buffer+cur, n-cur, "Aller à %s", m->t->name);
		}
		else
		{
			transform_t* tr = &m->t->harvest;

			if (tr->n_res == 0 || tr->res[0].is_item)
				return cur;

			int id = tr->res[0].id;
			kindOf_material_t* t = &u->materials[id];
			int skill = t->skill;

			float amount = c->inventory.materials[id];
			float max = character_maxOfMaterial(c, t);
			cur += snprintf(buffer+cur, n-cur, "%s (%.0f/%.0f)", u->skills[skill].name, floor(amount), floor(max));
		}
	}
	else if (o->t == O_CHARACTER)
	{
		character_t* t = (character_t*) o;
		const char* name = t->ai == NULL ? "ennemi" : t->ai->name;
		if (t == c)
			cur += snprintf(buffer+cur, n-cur, "S'admirer");
		else if (c->attack)
			cur += snprintf(buffer+cur, n-cur, "Attaquer %s", name);
		else
			cur += snprintf(buffer+cur, n-cur, "Suivre %s", name);
	}
	else if (o->t == O_BUILDING)
	{
		building_t* b = (building_t*) o;
		kindOf_building_t* t = b->t;
		const char* name = t->name;
		if (c->attack)
		{
			cur += snprintf(buffer+cur, n-cur, "Attaquer %s", name);
		}
		else if (moving)
		{
			cur += snprintf(buffer+cur, n-cur, "Se diriger vers %s", name);
		}
		else if (b->owner != c->o.uuid)
		{
			if (b->open)
				cur += snprintf(buffer+cur, n-cur, "Inspecter la marchandise");
			else
				cur += snprintf(buffer+cur, n-cur, "Attendre à l'abri des intempéries");
		}
		else if (b->build_progress != 1)
		{
			cur += snprintf(buffer+cur, n-cur, "%s", u->skills[SK_BUILD].name);
		}
		else if (b->work_n > 0)
		{
			transform_t* tr = &t->items[b->work_list[0]];

			if (tr->n_res == 0 || !tr->res[0].is_item)
				return cur;

			int id = tr->res[0].id;
			kindOf_item_t* it = &u->items[id];

			cur += snprintf(buffer+cur, 1024-cur, "%s (%i%%)", it->name, (int) floor(100*b->work_progress));
		}
		else if (t->make.n_res != 0)
		{
			transform_t* tr = &t->make;

			if (tr->n_res == 0 || tr->res[0].is_item)
				return cur;

			int id = tr->res[0].id;
			kindOf_material_t* t = &u->materials[id];
			int skill = t->skill;

			float amount = c->inventory.materials[id];
			float max = character_maxOfMaterial(c, t);
			cur += snprintf(buffer+cur, n-cur, "%s (%.0f/%.0f)", u->skills[skill].name, floor(amount), floor(max));
		}
		else
			cur += snprintf(buffer+cur, n-cur, "Se tourner les pources");
	}

	return cur;
}
