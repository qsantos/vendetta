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

#include "menu.h"

#include <unistd.h>
#include <math.h>

#include "widgets.h"
#include "game.h"

static char mainmenu(settings_t* s, graphics_t* gr, assets_t* a, char do_draw)
{
	(void) s;

	sfVector2u size = sfRenderWindow_getSize(gr->render);
	float x = size.x / 2;
	float y = size.y / 2 - 100;
	const char* labels[] =
	{
		"Jouer !",
		"Configurer",
		"Charger",
		"Quitter",
	};

	for (size_t i = 0; i < sizeof(labels)/sizeof(labels[0]); i++)
	{
		char enabled = i != 2 || !access("game.save", R_OK);
		if (draw_button(gr, a, x, y, labels[i], enabled, do_draw))
			return i;

		y += 50;
	}

	return -1;
}
static char configmenu(settings_t* s, graphics_t* gr, assets_t* a, char do_draw)
{
	sfVector2u size = sfRenderWindow_getSize(gr->render);
	float x = size.x / 2;
	float y = size.y / 2 - 100;

	sfText* text = NULL;
	if (text == NULL)
	{
		text = sfText_create();
		sfText_setFont(text, a->font);
		sfText_setCharacterSize(text, 20);
	}
	char buffer[1024];
	snprintf(buffer, 1024, "Seed: %#x", s->seed);
	sfText_setUTF8(text, buffer);
	sfFloatRect rect = sfText_getLocalBounds(text);
	sfVector2f pos = {floor(x-rect.width/2-rect.left), floor(y-rect.height/2-rect.top)};
	sfText_setPosition(text, pos);
	sfRenderWindow_drawText(gr->render, text, NULL);

	draw_slider(gr, a, x, (y+=50), "Largeur", &s->map_width,  100, 2000, do_draw);
	draw_slider(gr, a, x, (y+=50), "Height",  &s->map_height, 100, 2000, do_draw);
	draw_slider(gr, a, x, (y+=50), "Bots",    &s->bots_count,  10, 2000, do_draw);

	draw_toggle(gr, a, x, (y+=50), "Quickstart", &s->quickstart, do_draw);
	draw_toggle(gr, a, x, (y+=50), "Godmode",    &s->godmode,    do_draw);

	if (draw_button(gr, a, x, (y+=50), "Confirmer", 1, do_draw))
		return 0;

	return -1;
}
static void play(settings_t* s, graphics_t* gr, assets_t* a, char load)
{
	game_t game;
	game_init(&game, s, gr, a, load);
	game_loop(&game);
	game_exit(&game);
}
void menu(settings_t* s)
{
	graphics_t graphics;
	graphics_t* gr = &graphics;
	graphics_init(gr);

	assets_t assets;
	assets_t* a = &assets;
	assets_init(a);

	sfSprite* illustration = assets_sprite(a, "data/menu.png");

	sfRenderWindow* render = gr->render;
	char inconfig = 0;
	char stayhere = 1;
	while (stayhere && sfRenderWindow_isOpen(render))
	{
		sfEvent event;
		while (stayhere && sfRenderWindow_pollEvent(render, &event))
		{
			if (event.type == sfEvtClosed)
			{
				stayhere = 0;
			}
			else if (event.type == sfEvtResized)
			{
				sfFloatRect rect = {0,0,event.size.width,event.size.height};
				sfView* view = sfView_createFromRect(rect);
				sfRenderWindow_setView(render, view);
				sfView_destroy(view);
			}
			else if (event.type == sfEvtKeyReleased)
			{
				if (event.key.code == sfKeyEscape)
				{
					if (inconfig)
						inconfig = 0;
					else
						stayhere = 0;
				}
				else if (event.key.code == sfKeyReturn)
					play(s, gr, a, 0);
				else if (event.key.code == sfKeyC)
					play(s, gr, a, 1);
			}
			else if (event.type == sfEvtMouseButtonReleased)
			{
				if (inconfig)
				{
					if (configmenu(s, gr, a, 0) == 0)
						inconfig = 0;
				}
				else
				{
					int i = mainmenu(s, gr, a, 0);
					if (i == 0)
						play(s, gr, a, 0);
					else if (i == 1)
						inconfig = 1;
					else if (i == 2)
						play(s, gr, a, 1);
					else if (i == 3)
						stayhere = 0;
				}
			}
		}

		sfRenderWindow_clear(render, sfBlack);

		sfVector2u size  = sfRenderWindow_getSize(render);
		sfFloatRect rect = sfSprite_getGlobalBounds(illustration);
		sfVector2f pos = {size.x/2 - rect.width/2, size.y/2-rect.height/2};
		sfSprite_setPosition(illustration, pos);
		sfRenderWindow_drawSprite(render, illustration, NULL);

		if (inconfig)
			configmenu(s, gr, a, 1);
		else
			mainmenu(s, gr, a, 1);

		draw_cursor(gr, a, 0);
		sfRenderWindow_display(render);
	}

	graphics_exit(gr);
}
