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

#include "swequipment.h"

void swequipment_init(swequipment_t* w, graphics_t* g)
{
	subwindow_init(&w->w, g, "Equipment", 1024-SW_WIDTH*3, SW_HEIGHT);
}

void swequipment_exit(swequipment_t* w)
{
	subwindow_exit(&w->w);
}

int swequipment_draw(swequipment_t* w, game_t* g, char do_draw)
{
	if (do_draw)
	{
		if (!subwindow_draw(&w->w, g->g))
			return -1;
	}

	sfVector2f cursor;
	if (!do_draw)
		cursor = subwindow_mouse(&w->w, g->g);

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

	// keep track of two-handed weapons (take two slots)
	char twohanded = 0;
	for (size_t i = 0; i < g->u->n_slots; i++)
	{
		kindOf_slot_t* s = &g->u->slots[i];

		sfText_setPosition(text, pos);
		sfText_setUTF8(text, s->name); // TODO
		sfRenderWindow_drawText(g->g->render, text, NULL);

		int id = g->player->equipment[i];
		char* txt = "";
		if (id >= 0)
		{
			kindOf_item_t* it = &g->u->items[id];
			if (it->category == 1)
				twohanded = 1;
			txt = it->name;
		}
		else if (s->category == 0 && twohanded)
		{
			twohanded = 0;
			txt = "-";
		}

		pos.x += 150;
		sfText_setPosition(text, pos);
		pos.x -= 150;
		pos.y += 20;
		sfText_setUTF8(text, txt); // TODO
		if (do_draw)
		{
			sfRenderWindow_drawText(g->g->render, text, NULL);
			continue;
		}

		sfFloatRect rect = sfText_getGlobalBounds(text);
		if (sfFloatRect_contains(&rect, cursor.x, cursor.y))
			return i;
	}

	if (do_draw)
		sfRenderWindow_setView(g->g->render, g->g->overlay_view);

	return -1;
}

char swequipment_cursor(swequipment_t* w, game_t* g, int x, int y)
{
	if (!subwindow_cursor(&w->w, x, y))
		return 0;

	(void) g;

	return 1;
}

char swequipment_catch(swequipment_t* w, game_t* g, int x, int y, int t)
{
	if (!subwindow_cursor(&w->w, x, y))
		return 0;

	if (t != sfMouseLeft)
		return subwindow_catch(&w->w, x, y, t);

	int i = swequipment_draw(w, g, 0);
	if (i < 0)
		return subwindow_catch(&w->w, x, y, t);

	int j = g->player->equipment[i];
	if (j >= 0)
	{
		g->player->equipment[i] = -1;
		g->player->inventory.items[j]++;
	}
	return 1;
}
