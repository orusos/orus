/* The installer's step list: the Orus OS mark and the steps, in the desktop's colors.
 * Replaces the stock sidebar, which carries an "About Calamares" button. */
import io.calamares.ui 1.0
import io.calamares.core 1.0
import QtQuick 2.3
import QtQuick.Layouts 1.3

Rectangle {
    id: sideBar
    color: Branding.styleString(Branding.SidebarBackground)

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        Image {
            Layout.topMargin: 16
            Layout.bottomMargin: 12
            Layout.alignment: Qt.AlignHCenter
            width: 64
            height: 64
            source: "file:/" + Branding.imagePath(Branding.ProductLogo)
            sourceSize.width: width
            sourceSize.height: height
        }

        Repeater {
            model: ViewManager
            Rectangle {
                Layout.fillWidth: true
                Layout.leftMargin: 10
                Layout.rightMargin: 10
                height: 38
                radius: 10
                color: index == ViewManager.currentStepIndex
                    ? Branding.styleString(Branding.SidebarBackgroundCurrent)
                    : "transparent"
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.left: parent.left
                    anchors.leftMargin: 14
                    text: display
                    color: index == ViewManager.currentStepIndex
                        ? Branding.styleString(Branding.SidebarTextCurrent)
                        : Branding.styleString(Branding.SidebarText)
                    font.pixelSize: 15
                }
            }
        }

        Item {
            Layout.fillHeight: true
        }

        Text {
            Layout.alignment: Qt.AlignHCenter
            Layout.bottomMargin: 14
            text: "Version " + Branding.string(Branding.Version)
            color: Branding.styleString(Branding.SidebarText)
            opacity: 0.6
            font.pixelSize: 11
        }
    }
}
