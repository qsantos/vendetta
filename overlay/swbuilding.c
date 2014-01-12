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
#include <math.h>

#include "../util.h"

#define PANEL_N_COLS 3

void swbuilding_init(swbuilding_t* w, graphics_t* g)
{
	subwindow_init(&w->w, g, L"Building", 1024-SW_WIDTH*3, 0);
}

void swbuilding_exit(swbuilding_t* w)
{
	subwindow_exit(&w->w);
}

size_t component_tooltip(wchar_t* buffer, size_t n, universe_t* u, component_t* c)
{
	if (c->amount == 0)
		return 0;
	wchar_t* name = c->is_item ? u->items[c->id].name: u->materials[c->id].name;
	return swprintf(buffer, n, L"\n%.1f %ls", floor(c->amount), name);
}
size_t swbuilding_tooltip(wchar_t* buffer, size_t n, universe_t* u, transform_t* tr)
{
	size_t cur = 0;

	// result components
	cur += swprintf(buffer+cur, n-cur, L"make:");
	for (int i = 0; i < tr->n_res; i++)
		cur += component_tooltip(buffer+cur, n-cur, u, &tr->res[i]);

	// required components
	if (tr->n_req != 0)
		cur += swprintf(buffer+cur, n-cur, L"\n\nneed:");
	for (int i = 0; i < tr->n_req; i++)
		cur += component_tooltip(buffer+cur, n-cur, u, &tr->req[i]);

	return cur;
}

char swbuilding_cursor(swbuilding_t* w, game_t* g, int _x, int _y)
{
	if (!w->w.visible)
		return 0;

	building_t* b = g->player->inBuilding;
	if (b == NULL)
		return 0;

	sfVector2f cursor = sfRenderWindow_mapPixelToCoords(g->g->render, (sfVector2i){_x,_y}, w->w.view);
	float x = cursor.x;
	float y = cursor.y;

	static sfText* text = NULL;
	if (text == NULL)
	{
		text = sfText_create();
		sfText_setFont         (text, g->g->font);
		sfText_setCharacterSize(text, 18);
	}

	sfVector2f pos = {0, 0};
	pos.y += 20; // material

	kindOf_building_t* t = b->t;
	if (t->make.n_res != 0)
	{
		component_t* c = &t->make.res[0];
		if (c->is_item)
			exit(1);

		const wchar_t* action = t->make.n_req == 0 ? L"Harvest" : L"Transform to";
		const wchar_t* name   = g->u->materials[c->id].name;
		wchar_t buffer[1024];
		swprintf(buffer, 1024, L"%ls %ls", action, name);

		sfText_setPosition(text, pos);
		sfText_setWString(text, buffer);

		sfFloatRect rect = sfText_getGlobalBounds(text);
		if (sfFloatRect_contains(&rect, x, y))
		{
			wchar_t buffer[1024];
			swbuilding_tooltip(buffer, 1024, g->u, &t->make);
			graphics_drawTooltip(g->g, _x, _y, buffer);
			return 1;
		}
	}
	pos.y += 20;

	for (int i = 0; i < t->n_items; i++)
	{
		transform_t* tr = &t->items[i];
		if (tr->n_res == 0)
			continue;

		component_t* c = &tr->res[0];
		if (!c->is_item)
			exit(1);

		pos.y += 20;

		wchar_t* name = g->u->items[c->id].name;
		sfText_setPosition(text, pos);
		sfText_setWString(text, name);

		sfFloatRect rect = sfText_getGlobalBounds(text);
		if (!sfFloatRect_contains(&rect, x, y))
			continue;

		wchar_t buffer[1024];
		swbuilding_tooltip(buffer, 1024, g->u, tr);
		graphics_drawTooltip(g->g, _x, _y, buffer);
		return 1;
	}

	return 0;
}

void swbuilding_draw(swbuilding_t* w, game_t* g)
{
	if (!subwindow_draw(&w->w, g->g))
		return;

	building_t* b = g->player->inBuilding;
	if (b == NULL)
	{
		sfRenderWindow_setView(g->g->render, g->g->overlay_view);
		return;
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

	sfVector2f pos = {0, 0};

	wchar_t buffer[1024];
	swprintf(buffer, 1024, L"%ls", t->name);

	sfText_setPosition(text, pos);
	sfText_setWString(text, buffer);
	sfRenderWindow_drawText(g->g->render, text, NULL);
	pos.y += 20;

	if (t->make.n_res != 0)
	{
		component_t* c = &t->make.res[0];
		if (c->is_item)
			exit(1);

		const wchar_t* action = t->make.n_req == 0 ? L"Harvest" : L"Transform to";
		const wchar_t* name   = g->u->materials[c->id].name;
		wchar_t buffer[1024];
		swprintf(buffer, 1024, L"%ls %ls", action, name);

		sfText_setPosition(text, pos);
		sfText_setWString(text, buffer);
		sfRenderWindow_drawText(g->g->render, text, NULL);
	}
	pos.y += 20;

	int cur_work = b->work_n == 0 ? -1 : b->work_list[0];
	for (int i = 0; i < t->n_items; i++)
	{
		transform_t* tr = &t->items[i];
		if (tr->n_res == 0)
			continue;

		component_t* c = &tr->res[0];
		if (!c->is_item)
			exit(1);

		pos.y += 20;

		wchar_t* name = g->u->items[c->id].name;
		wchar_t buffer[1024];
		if (i == cur_work)
			swprintf(buffer, 1024, L"%ls (%i%%)", name, (int) floor(100*b->work_progress));
		else
			swprintf(buffer, 1024, L"%ls", name);

		sfText_setPosition(text, pos);
		sfText_setWString(text, buffer);
		sfRenderWindow_drawText(g->g->render, text, NULL);
	}

//	sfText_destroy(text); // TODO

	sfRenderWindow_setView(g->g->render, g->g->overlay_view);
}

char swbuilding_catch(swbuilding_t* w, game_t* g, int _x, int _y, int t)
{
	if (!subwindow_cursor(&w->w, _x, _y))
		return 0;

	building_t* b = g->player->inBuilding;
	if (b == NULL)
		return 1;

	if (t != sfMouseLeft)
		return 1;

	sfVector2f cursor = sfRenderWindow_mapPixelToCoords(g->g->render, (sfVector2i){_x,_y}, w->w.view);
	float x = cursor.x;
	float y = cursor.y;

	static sfText* text = NULL;
	if (text == NULL)
	{
		text = sfText_create();
		sfText_setFont         (text, g->g->font);
		sfText_setCharacterSize(text, 18);
	}

	sfVector2f pos = {0, 0};
	pos.y += 20; // building name
	pos.y += 20; // material

	for (int i = 0; i < b->t->n_items; i++)
	{
		transform_t* tr = &b->t->items[i];
		if (tr->n_res == 0)
			continue;

		component_t* c = &tr->res[0];
		if (!c->is_item)
			exit(1);

		pos.y += 20;

		wchar_t* name = g->u->items[c->id].name;
		sfText_setPosition(text, pos);
		sfText_setWString(text, name);

		sfFloatRect rect = sfText_getGlobalBounds(text);
		if (!sfFloatRect_contains(&rect, x, y))
			continue;

		if (transform_check(tr, &g->player->inventory))
			building_work_enqueue(b, i);

		return 1;
	}

	return subwindow_catch(&w->w, _x, _y, t);
}
