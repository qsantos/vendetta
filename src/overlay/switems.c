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

#include "switems.h"

#include <stdio.h>
#include <math.h>

void switems_init(switems_t* w, graphics_t* g)
{
	subwindow_init(&w->w, g, "Objets", 1024-SW_WIDTH*2, 0);
}

void switems_exit(switems_t* w)
{
	subwindow_exit(&w->w);
}

size_t switems_itemTooltip(char* buffer, size_t n, universe_t* u, kindOf_item_t* it)
{
	size_t cur = 0;

	cur += snprintf(buffer+cur, n-cur, "%s", it->name);

	effect_t* e = &it->effect;
	for (size_t i = 0; i < e->n_skills; i++)
	{
		int skill = e->skills[i];
		kindOf_skill_t* k = &u->skills[skill];
		cur += snprintf(buffer+cur, n-cur, "\n%+.1f %s", e->bonuses[i], k->name);
	}

	return cur;
}

int switems_draw(switems_t* w, game_t* g, char do_draw)
{
	if (do_draw)
	{
		if (!subwindow_draw(&w->w, g->g))
			return -1;
	}

	sfVector2f mouse;
	if (!do_draw)
		mouse = subwindow_mouse(&w->w, g->g);

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
	for (size_t i = 0; i < g->u->n_items; i++)
	{
		float amount = floor(g->player->inventory.items[i]);

		if (amount == 0)
			continue;

		kindOf_item_t* it = &g->u->items[i];

		// icon
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
		snprintf(buffer, 1024, "%s: %.0f", it->name, amount);
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

	if (do_draw)
		sfRenderWindow_setView(g->g->render, g->g->overlay_view);

	w->w.height = y;
	return -1;
}

int switems_cursor(switems_t* w, game_t* g, int x, int y)
{
	if (!subwindow_cursor(&w->w, x, y))
		return -1;

	int i = switems_draw(w, g, 0);
	if (i < 0)
		return 0;

	char buffer[1024];
	switems_itemTooltip(buffer, 1024, g->u, &g->u->items[i]);
	graphics_drawTooltip(g->g, x, y, buffer);

	return 12;
}

char switems_catch(switems_t* w, game_t* g, int x, int y, int t)
{
	if (!subwindow_cursor(&w->w, x, y))
		return 0;

	if (t != sfMouseLeft)
		return subwindow_catch(&w->w, x, y, t);

	int i = switems_draw(w, g, 0);
	if (i < 0)
		return subwindow_catch(&w->w, x, y, t);

	int cat = g->u->items[i].category;

	// TODO
	// BEGIN quickfix
	// handling two-handed items simply is tricky
	// as there is only one such category, let's hard-code it!
	// two-handed items are equiped in single-handed slot and reserve the next one
	if (cat == 0) // one-handed
	{
		char twohanded = 0; // keep track of reserved slot
		for (size_t j = 0; j < g->u->n_slots; j++)
			if (g->u->slots[j].category == 0)
			{
				int k = g->player->equipment[j];
				if (k < 0)
				{
					if (twohanded)
					{
						twohanded = 0;
					}
					else
					{
						g->player->equipment[j] = i;
						g->player->inventory.items[i]--;
						break;
					}
				}
				else if (g->u->items[k].category == 1)
				{
					twohanded = 1;
				}
			}
	}
	else if (cat == 1) // two-handed
	{
		// find two single-hand slots
		int a = -1;
		int b = -1;
		for (size_t j = 0; j < g->u->n_slots; j++)
			if (g->u->slots[j].category == 0 && g->player->equipment[j] < 0)
			{
				if (a < 0)
					a = j;
				else
				{
					b = j;
					break;
				}
			}
		if (a >= 0 && b >= 0)
		{
			g->player->equipment[a] = i;
			g->player->inventory.items[i]--;
		}
	}
	else
	// END quickfix

	for (size_t j = 0; j < g->u->n_slots; j++)
		if (g->u->slots[j].category == cat && g->player->equipment[j] < 0)
		{
			g->player->equipment[j] = i;
			g->player->inventory.items[i]--;
			break;
		}

	return 1;
}
