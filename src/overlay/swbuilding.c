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

void swbuilding_init(swbuilding_t* w, graphics_t* g)
{
	subwindow_init(&w->w, g, "Building", 1024-SW_WIDTH*3, 0);
}

void swbuilding_exit(swbuilding_t* w)
{
	subwindow_exit(&w->w);
}

size_t component_tooltip(char* buffer, size_t n, universe_t* u, component_t* c)
{
	if (c->amount == 0)
		return 0;
	char* name = c->is_item ? u->items[c->id].name: u->materials[c->id].name;
	return snprintf(buffer, n, "\n%.1f %s", floor(c->amount), name);
}
size_t swbuilding_tooltip(char* buffer, size_t n, universe_t* u, transform_t* tr)
{
	size_t cur = 0;

	// result components
	cur += snprintf(buffer+cur, n-cur, "make:");
	for (int i = 0; i < tr->n_res; i++)
		cur += component_tooltip(buffer+cur, n-cur, u, &tr->res[i]);

	// required components
	if (tr->n_req != 0)
		cur += snprintf(buffer+cur, n-cur, "\n\nneed:");
	for (int i = 0; i < tr->n_req; i++)
		cur += component_tooltip(buffer+cur, n-cur, u, &tr->req[i]);

	return cur;
}

int swbuilding_draw(swbuilding_t* w, game_t* g, char do_draw)
{
	if (do_draw)
	{
		if (!subwindow_draw(&w->w, g->g))
			return -1;
	}

	sfVector2f mouse;
	if (!do_draw)
		mouse = subwindow_mouse(&w->w, g->g);

	building_t* b = g->player->inBuilding;
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
		sfText_setFont         (text, g->g->font);
		sfText_setCharacterSize(text, 15);
		sfText_setColor        (text, color);
	}

	float x = 0;
	float y = 0;
	char caught = 0;

	char buffer[1024];
	snprintf(buffer, 1024, "%s", t->name);
	sfText_setPosition(text, (sfVector2f){x,y});
	sfText_setUTF8(text, buffer);
	if (do_draw)
		sfRenderWindow_drawText(g->g->render, text, NULL);

	y += 20;

	if (t->make.n_res != 0)
	{
		component_t* c = &t->make.res[0];
		if (c->is_item)
			exit(1);

		kindOf_material_t* m = &g->u->materials[c->id];

		// icon
		sfSprite* sprite = g->g->sprites[m->icon_sprite];
		sfIntRect rect = {32*m->icon_index, 0, 32, 32};
		sfSprite_setTextureRect(sprite, rect);
		sfSprite_setPosition(sprite, (sfVector2f){x,y});
		if (do_draw)
			sfRenderWindow_drawSprite(g->g->render, sprite, NULL);

		// text
		const char* action = t->make.n_req == 0 ? "Harvest" : "Transform to";
		char buffer[1024];
		snprintf(buffer, 1024, "%s %s", action, m->name);
		sfText_setPosition(text, (sfVector2f){x+32,y+6});
		sfText_setUTF8(text, buffer);
		if (do_draw)
			sfRenderWindow_drawText(g->g->render, text, NULL);

		y += 32;
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
		sfSprite* sprite = g->g->sprites[it->icon_sprite];
		sfIntRect rect = {32*col, 32*row, 32, 32};
		sfSprite_setTextureRect(sprite, rect);
		sfSprite_setPosition(sprite, (sfVector2f){x,y});
		if (do_draw)
			sfRenderWindow_drawSprite(g->g->render, sprite, NULL);
		else
			caught |= sfSprite_contains(sprite, mouse);

		// text
		char buffer[1024];
		if ((int) i == cur_work)
			snprintf(buffer, 1024, "%s (%i%%)", it->name, (int) floor(100*b->work_progress));
		else
			snprintf(buffer, 1024, "%s", it->name);
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

int swbuilding_cursor(swbuilding_t* w, game_t* g, int x, int y)
{
	if (!subwindow_cursor(&w->w, x, y))
		return -1;

	int i = swbuilding_draw(w, g, 0);
	if (i < 0)
		return 0;

	char buffer[1024];
	building_t* b = g->player->inBuilding;
	transform_t* tr = &b->t->items[i];
	swbuilding_tooltip(buffer, 1024, g->u, tr);
	graphics_drawTooltip(g->g, x, y, buffer);
	return 2;
}

char swbuilding_catch(swbuilding_t* w, game_t* g, int x, int y, int t)
{
	if (!subwindow_cursor(&w->w, x, y))
		return 0;

	if (t != sfMouseLeft)
		return subwindow_catch(&w->w, x, y, t);

	int i = swbuilding_draw(w, g, 0);
	if (i < 0)
		return subwindow_catch(&w->w, x, y, t);

	building_t* b = g->player->inBuilding;
	transform_t* tr = &b->t->items[i];
	if (transform_check(tr, &g->player->inventory))
		building_work_enqueue(b, i);

	return 1;
}
