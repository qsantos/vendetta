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

#include "swbuilding.h"

#include <stdlib.h>
#include <stdio.h>
#include <math.h>

#include "../widgets.h"

void swbuilding_init(swbuilding_t* w, game_t* g)
{
	subwindow_init(&w->w, g->g, "Bâtiment", 1024-SW_WIDTH*3, 0);
}

void swbuilding_exit(swbuilding_t* w)
{
	subwindow_exit(&w->w);
}

size_t swbuilding_tooltip(char* buffer, size_t n, universe_t* u, transform_t* tr)
{
	size_t cur = 0;

	// result components
	cur += snprintf(buffer+cur, n-cur, "Fabriquer ");
	for (int i = 0; i < tr->n_res; i++)
	{
		component_t* c = &tr->res[i];

		if (c->is_item)
			cur += kindOf_item_info(&u->items[c->id], buffer+cur, n-cur, u);
		else
			cur += kindOf_material_info(&u->materials[c->id], buffer+cur, n-cur, u);

		if (c->amount != 1)
			cur += snprintf(buffer+cur, n-cur, " (×%.0f)", floor(c->amount));
		cur += snprintf(buffer+cur, n-cur, "\n\n");
	}

	// required components
	if (tr->n_req != 0)
		cur += snprintf(buffer+cur, n-cur, "Requiert");
	for (int i = 0; i < tr->n_req; i++)
	{
		component_t* c = &tr->req[i];
		char* name = c->is_item ? u->items[c->id].name: u->materials[c->id].name;
		cur += snprintf(buffer+cur, n-cur, "\n%.1f %s", floor(c->amount), name);
	}

	return cur;
}

int swbuilding_draw(swbuilding_t* w, game_t* g, char do_draw)
{
	if (do_draw)
	{
		if (!subwindow_draw(&w->w, g->g, g->a))
			return -1;
	}

	sfVector2f mouse;
	if (!do_draw)
		mouse = subwindow_mouse(&w->w, g->g);

	building_t* b = building_get(&g->w->objects, g->player->inBuilding);
	if (b == NULL)
	{
		if (do_draw)
			sfRenderWindow_setView(g->g->render, g->g->overlay_view);

		w->w.height = 0;
		return -1;
	}

	kindOf_building_t* t = b->t;

	static sfText* text = NULL;
	if (text == NULL)
	{
		sfColor color = {255, 255, 255, 255};

		text = sfText_create();
		sfText_setFont         (text, g->a->font);
		sfText_setCharacterSize(text, 15);
		sfText_setColor        (text, color);
	}

	float x = 0;
	float y = 0;
	char caught = 0;

	char buffer[1024];
	size_t cur = 0;
	cur += snprintf(buffer+cur, 1024-cur, "%s", t->name);
	float m = b->inventory.money;
	if (m >= 1)
		snprintf(buffer+cur, 1024-cur, " (%.0f or)", floor(m));
	sfText_setPosition(text, (sfVector2f){x,y});
	sfText_setUTF8(text, buffer);
	if (do_draw)
		sfRenderWindow_drawText(g->g->render, text, NULL);
	else
		caught |= sfText_contains(text, mouse);

	if (caught)
		return -3;

	y += 20;

	if (t->make.n_res != 0)
	{
		component_t* c = &t->make.res[0];
		if (c->is_item)
			exit(1);

		kindOf_material_t* m = &g->u->materials[c->id];

		// icon
		sfSprite* sprite = g->a->sprites[m->icon_sprite];
		sfIntRect rect = {32*m->icon_index, 0, 32, 32};
		sfSprite_setTextureRect(sprite, rect);
		sfSprite_setPosition(sprite, (sfVector2f){x,y});
		if (do_draw)
			sfRenderWindow_drawSprite(g->g->render, sprite, NULL);
		else
			caught |= sfSprite_contains(sprite, mouse);

		// text
		char buffer[1024];
		size_t cur = 0;
		size_t n = 1024;
		if (b->owner != g->player->o.uuid)
		{
			cur += snprintf(buffer+cur, n-cur, "Acheter %s", m->name);
		}
		else if (t->make.n_req == 0)
		{
			cur += snprintf(buffer+cur, n-cur, "Récolter %s", m->name);
		}
		else
		{
			kindOf_material_t* x = &g->u->materials[t->make.req[0].id];
			cur += snprintf(buffer+cur, n-cur, "%s vers %s", x->name, m->name);
		}

		float q = b->inventory.materials[c->id];
		cur += snprintf(buffer+cur, n-cur, " (%.0f)", floor(q));

		sfText_setPosition(text, (sfVector2f){x+32,y+6});
		sfText_setUTF8(text, buffer);
		if (do_draw)
			sfRenderWindow_drawText(g->g->render, text, NULL);
		else
			caught |= sfText_contains(text, mouse);

		y += 32;

		if (caught)
			return -2;
	}

	y += 20;

	int cur_work = b->work_n == 0 ? -1 : b->work_list[0];
	for (size_t i = 0; i < t->n_items; i++)
	{
		transform_t* tr = &t->items[i];
		if (tr->n_res == 0)
			continue;

		component_t* c = &tr->res[0];
		if (!c->is_item)
			exit(1);

		// icon
		kindOf_item_t* it = &g->u->items[c->id];
		static const int max_cols = 1024 / 32;
		int idx = it->icon_index;
		int col = idx % max_cols;
		int row = idx / max_cols;
		sfSprite* sprite = g->a->sprites[it->icon_sprite];
		sfIntRect rect = {32*col, 32*row, 32, 32};
		sfSprite_setTextureRect(sprite, rect);
		sfSprite_setPosition(sprite, (sfVector2f){x,y});
		if (do_draw)
			sfRenderWindow_drawSprite(g->g->render, sprite, NULL);
		else
			caught |= sfSprite_contains(sprite, mouse);

		// text
		char buffer[1024];
		size_t cur = 0;
		cur += snprintf(buffer+cur, 1024-cur, "%s", it->name);

		int n = (int) floor(b->inventory.items[c->id]);
		if (n != 0)
			cur += snprintf(buffer+cur, 1024-cur, " (×%i)", n);

		if ((int) i == cur_work)
			cur += snprintf(buffer+cur, 1024-cur, " (%i%%)", (int) floor(100*b->work_progress));

		sfText_setPosition(text, (sfVector2f){x+32,y+6});
		sfText_setUTF8(text, buffer);
		if (do_draw)
			sfRenderWindow_drawText(g->g->render, text, NULL);
		else
			caught |= sfText_contains(text, mouse);

		y += 32;

		if (caught)
			return i;
	}

//	sfText_destroy(text); // TODO

	if (do_draw)
		sfRenderWindow_setView(g->g->render, g->g->overlay_view);

	w->w.height = y;
	return -1;
}

int swbuilding_cursor(swbuilding_t* w, game_t* g)
{
	if (!subwindow_cursor(&w->w, g->g))
		return -1;

	int i = swbuilding_draw(w, g, 0);
	if (i == -1 || i == -3)
		return 0;

	char buffer[1024];
	building_t* b = building_get(&g->w->objects, g->player->inBuilding);
	transform_t* tr = i >= 0 ? &b->t->items[i] : &b->t->make;
	swbuilding_tooltip(buffer, 1024, g->u, tr);
	draw_tooltip(g->g, g->a, buffer);
	return 2;
}

char swbuilding_catch(swbuilding_t* w, game_t* g, int t)
{
	if (!subwindow_cursor(&w->w, g->g))
		return 0;

	// button pressed
	if (t < 0)
		return subwindow_catch(&w->w, g->g, t);

	int i = swbuilding_draw(w, g, 0);
	if (i == -1)
		return subwindow_catch(&w->w, g->g, t);

	character_t* c = g->player;
	building_t* b = building_get(&g->w->objects, c->inBuilding);
	char isOwner = b->owner == c->o.uuid;

	// take money
	if (i == -3)
	{
		if (isOwner)
			building_withdraw(b, &c->inventory);
		return 1;
	}

	// take item
	transform_t* tr = i >= 0 ? &b->t->items[i] : &b->t->make;
	if (!isOwner || t == sfMouseRight)
	{
		component_t* k = &tr->res[0];
		building_take(b, k->is_item, k->id, 1.0f, &c->inventory, isOwner);
	}
	else if (t == sfMouseLeft)
	{
		if (transform_check(tr, &c->inventory))
			building_work_enqueue(b, i);
	}

	return 1;
}
